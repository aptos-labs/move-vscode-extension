//! See [`Parser`].

use drop_bomb::DropBomb;

use crate::parse::event::Event;
use crate::parse::text_token_source::TextTokenSource;
use crate::parse::token_set::TokenSet;
use crate::parse::ParseError;
use crate::{
    SyntaxKind::{self, EOF, ERROR, TOMBSTONE},
    T,
};

/// `Parser` struct provides the low-level API for
/// navigating through the stream of tokens and
/// constructing the parse tree. The actual parsing
/// happens in the [`grammar`](super::grammar) module.
///
/// However, the result of this `Parser` is not a real
/// tree, but rather a flat stream of events of the form
/// "start expression, consume number literal,
/// finish expression". See `Event` docs for more.
pub struct Parser<'t> {
    token_source: &'t mut TextTokenSource<'t>,
    events: Vec<Event>,
}

impl<'t> Parser<'t> {
    pub(super) fn new(token_source: &'t mut TextTokenSource<'t>) -> Parser<'t> {
        Parser {
            token_source,
            events: Vec::new(),
        }
    }

    pub(crate) fn finish(self) -> Vec<Event> {
        self.events
    }

    /// Returns the kind of the current token.
    /// If parser has already reached the end of input,
    /// the special `EOF` kind is returned.
    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    pub(crate) fn nth_is_jointed_to_next(&self, n: usize) -> bool {
        self.token_source.lookahead_nth(n).is_jointed_to_next
    }

    /// Lookahead operation: returns the kind of the next nth
    /// token.
    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        assert!(n <= 3);

        // let steps = self.steps.get();
        // assert!(PARSER_STEP_LIMIT.check(steps as usize).is_ok(), "the parser seems stuck");
        // self.steps.set(steps + 1);

        self.token_source.lookahead_nth(n).kind
    }

    /// Checks if the current token is `kind`.
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.nth_at(0, kind)
    }

    pub(crate) fn if_at(&mut self, kind: SyntaxKind, mut f: impl FnMut(&mut Parser)) {
        if self.at(kind) {
            f(self);
        } else {
            self.error(format!("expected {:?}", kind));
        }
    }

    pub(crate) fn nth_at_ts(&self, n: usize, ts: TokenSet) -> bool {
        let nth_kind = self.nth(n);
        ts.contains(nth_kind)
    }

    pub(crate) fn nth_at(&self, n: usize, kind: SyntaxKind) -> bool {
        match kind {
            // T![-=] => self.at_composite2(n, T![-], T![=]),
            // T![->] => self.at_composite2(n, T![-], T![>]),
            // T![::] => self.at_composite2(n, T![:], T![:]),
            // T![!=] => self.at_composite2(n, T![!], T![=]),
            // T![..] => self.at_composite2(n, T![.], T![.]),
            // T![*=] => self.at_composite2(n, T![*], T![=]),
            // T![/=] => self.at_composite2(n, T![/], T![=]),
            T![&&] => self.at_composite2(n, T![&], T![&]),
            // T![&=] => self.at_composite2(n, T![&], T![=]),
            // T![%=] => self.at_composite2(n, T![%], T![=]),
            // T![^=] => self.at_composite2(n, T![^], T![=]),
            // T![+=] => self.at_composite2(n, T![+], T![=]),
            T![<<] => self.at_composite2(n, T![<], T![<]),
            T![<=] => self.at_composite2(n, T![<], T![=]),
            // T![==] => self.at_composite2(n, T![=], T![=]),
            // T![=>] => self.at_composite2(n, T![=], T![>]),
            T![>=] => self.at_composite2(n, T![>], T![=]),
            T![>>] => self.at_composite2(n, T![>], T![>]),
            // T![|=] => self.at_composite2(n, T![|], T![=]),
            T![||] => self.at_composite2(n, T![|], T![|]),

            // T![...] => self.at_composite3(n, T![.], T![.], T![.]),
            // T![..=] => self.at_composite3(n, T![.], T![.], T![=]),
            T![<<=] => self.at_composite3(n, T![<], T![<], T![=]),
            T![>>=] => self.at_composite3(n, T![>], T![>], T![=]),

            T![<==>] => self.at_composite2(n, T![<], T![==>]),

            _ => self.token_source.lookahead_nth(n).kind == kind,
        }
    }

    /// Consume the next token if `kind` matches.
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if !self.at(kind) {
            return false;
        }
        let n_raw_tokens = match kind {
            // T![-=]
            // | T![->]
            // | T![::]
            // | T![!=]
            // | T![..]
            // | T![*=]
            // | T![/=]
            | T![&&]
            // | T![&=]
            // | T![%=]
            // | T![^=]
            // | T![+=]
            | T![<<]
            | T![<=]
            // | T![==]
            // | T![=>]
            | T![>=]
            | T![>>]
            // | T![|=]
            | T![||] => 2,

            /*T![...] | T![..=] |*/ T![<<=] | T![>>=] => 3,
            T![<==>] => 2,
            _ => 1,
        };
        self.do_bump(kind, n_raw_tokens);
        true
    }

    fn at_composite2(&self, n: usize, k1: SyntaxKind, k2: SyntaxKind) -> bool {
        let t1 = self.token_source.lookahead_nth(n);
        if t1.kind != k1 || !t1.is_jointed_to_next {
            return false;
        }
        let t2 = self.token_source.lookahead_nth(n + 1);
        t2.kind == k2
    }

    fn at_composite3(&self, n: usize, k1: SyntaxKind, k2: SyntaxKind, k3: SyntaxKind) -> bool {
        let t1 = self.token_source.lookahead_nth(n);
        if t1.kind != k1 || !t1.is_jointed_to_next {
            return false;
        }
        let t2 = self.token_source.lookahead_nth(n + 1);
        if t2.kind != k2 || !t2.is_jointed_to_next {
            return false;
        }
        let t3 = self.token_source.lookahead_nth(n + 2);
        t3.kind == k3
    }

    fn at_composite4(
        &self,
        n: usize,
        k1: SyntaxKind,
        k2: SyntaxKind,
        k3: SyntaxKind,
        k4: SyntaxKind,
    ) -> bool {
        let t1 = self.token_source.lookahead_nth(n);
        if t1.kind != k1 || !t1.is_jointed_to_next {
            return false;
        }
        let t2 = self.token_source.lookahead_nth(n + 1);
        if t2.kind != k2 || !t2.is_jointed_to_next {
            return false;
        }
        let t3 = self.token_source.lookahead_nth(n + 2);
        if t3.kind != k3 || !t3.is_jointed_to_next {
            return false;
        }
        let t4 = self.token_source.lookahead_nth(n + 3);
        t4.kind == k4
    }

    /// Checks if the current token is in `kinds`.
    pub(crate) fn at_ts(&self, kinds: TokenSet) -> bool {
        kinds.contains(self.current())
    }

    /// Checks if the current token is in `kinds`.
    pub(crate) fn at_ts_fn<Kinds>(&self, kinds: Kinds) -> bool
    where
        Kinds: Fn(&Parser) -> bool,
    {
        kinds(self)
        // kinds.contains(self.current())
    }

    pub(crate) fn at_contextual_kw_ident(&self, kw: &str) -> bool {
        self.at(T![ident]) && self.at_contextual_kw(kw)
    }

    /// Checks if the current token is contextual keyword with text `t`.
    pub(crate) fn at_contextual_kw(&self, kw: &str) -> bool {
        self.token_source.is_keyword(kw)
    }

    /// Starts a new node in the syntax tree. All nodes and tokens
    /// consumed between the `start` and the corresponding `Marker::complete`
    /// belong to the same node.
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len() as u32;
        self.push_event(Event::tombstone());
        Marker::new(pos)
    }

    /// Consume the next token if `kind` matches.
    #[track_caller]
    pub(crate) fn bump(&mut self, kind: SyntaxKind) {
        assert!(self.eat(kind), "actual token is {:?}", self.current());
    }

    /// Advances the parser by one token
    pub(crate) fn bump_any(&mut self) {
        let kind = self.nth(0);
        if kind == EOF {
            return;
        }
        self.do_bump(kind, 1);
    }

    /// Advances the parser by one token, remapping its kind.
    /// This is useful to create contextual keywords from
    /// identifiers. For example, the lexer creates a `union`
    /// *identifier* token, but the parser remaps it to the
    /// `union` keyword, and keyword is what ends up in the
    /// final tree.
    pub(crate) fn bump_remap(&mut self, kind: SyntaxKind) {
        if self.nth(0) == EOF {
            // FIXME: panic!?
            return;
        }
        self.do_bump(kind, 1);
    }

    pub(crate) fn bump_remap_many(&mut self, kind: SyntaxKind, n_tokens: u8) {
        if self.nth(0) == EOF {
            // FIXME: panic!?
            return;
        }
        self.do_bump(kind, n_tokens);
    }

    /// Emit error with the `message`.
    pub(crate) fn error(&mut self, message: impl Into<String>) {
        // self.error_and_bump_until(&message.into(), |p| false);
        self.push_error(message);
    }

    pub(crate) fn push_error(&mut self, message: impl Into<String>) {
        let msg = ParseError(Box::new(message.into()));
        self.push_event(Event::Error { msg });
    }

    /// Consume the next token if it is `kind` or emit an error
    /// otherwise.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.push_error(format!("expected {:?}", kind));
        // self.error(format!("expected {:?}", kind));
        false
    }

    // /// Create an error node and consume the next token.
    // pub(crate) fn error_and_bump(&mut self, message: &str) {
    //     self.err_recover_at_ts(message, TokenSet::EMPTY);
    // }

    /// Create an error node and consume the next token.
    pub(crate) fn error_and_bump_until_at_ts(&mut self, message: &str, stop_at_ts: TokenSet) {
        self.error_and_bump_until(message, |p| p.at_ts(stop_at_ts));
        // match self.current() {
        //     T!['{'] | T!['}'] => {
        //         self.error(message);
        //         return;
        //     }
        //     _ => (),
        // }
        //
        // if self.at_ts(recovery) {
        //     self.error(message);
        //     return;
        // }
        //
        // let m = self.start();
        // self.error(message);
        // self.bump_any();
        // m.complete(self, ERROR);
    }

    /// adds error and then bumps until `stop()` is true
    pub(crate) fn error_and_bump_until(&mut self, message: &str, stop: impl Fn(&Parser) -> bool) {
        self.push_error(message);
        self.bump_until(stop);
    }

    // pub(crate) fn with_recover_until(&mut self, f: impl Fn(&mut Parser)) {
    //     // self.stop_recovery = Some(Box::new(stop_recovery));
    //     f(self);
    //     // self.stop_recovery = None;
    // }

    pub(crate) fn bump_until(&mut self, stop: impl Fn(&Parser) -> bool) {
        while !self.at(EOF) {
            if stop(self) {
                break;
            }
            self.bump_any();
        }
    }

    pub(crate) fn error_and_bump_any(&mut self, message: &str) {
        let m = self.start();
        self.push_error(message);
        self.bump_any();
        m.complete(self, ERROR);
    }

    fn do_bump(&mut self, kind: SyntaxKind, n_raw_tokens: u8) {
        for _ in 0..n_raw_tokens {
            self.token_source.bump();
        }

        self.push_event(Event::Token { kind, n_raw_tokens });
    }

    fn rollback(&mut self) {
        if let Some(Event::Token { kind: _, n_raw_tokens }) = self.events.pop() {
            for _ in 0..n_raw_tokens {
                self.token_source.rollback();
            }
        }
    }

    fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }
}

/// See [`Parser::start`].
pub(crate) struct Marker {
    pos: u32,
    bomb: DropBomb,
}

impl Marker {
    fn new(pos: u32) -> Marker {
        Marker {
            pos,
            bomb: DropBomb::new("Marker must be either completed or abandoned"),
        }
    }

    /// Finishes the syntax tree node and assigns `kind` to it,
    /// and mark the create a `CompletedMarker` for possible future
    /// operation like `.precede()` to deal with forward_parent.
    pub(crate) fn complete(mut self, p: &mut Parser, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();
        let idx = self.pos as usize;
        match &mut p.events[idx] {
            Event::Start { kind: slot, .. } => {
                *slot = kind;
            }
            _ => unreachable!(),
        }
        p.push_event(Event::Finish);
        // let end_pos = p.events.len() as u32;
        CompletedMarker::new(self.pos /*, end_pos*/, kind)
    }

    /// Abandons the syntax tree node. All its children
    /// are attached to its parent instead.
    pub(crate) fn abandon(mut self, p: &mut Parser) {
        self.bomb.defuse();
        let idx = self.pos as usize;
        if idx == p.events.len() - 1 {
            match p.events.pop() {
                Some(Event::Start {
                    kind: TOMBSTONE,
                    forward_parent: None,
                }) => (),
                _ => unreachable!(),
            }
        }
    }

    /// Abandons the syntax tree node. All its children events are dropped and position restored.
    pub(crate) fn abandon_with_rollback(mut self, p: &mut Parser) {
        self.bomb.defuse();
        let idx = self.pos as usize;
        if idx == p.events.len() - 1 {
            match p.events.pop() {
                Some(Event::Start {
                    kind: TOMBSTONE,
                    forward_parent: None,
                }) => (),
                _ => unreachable!(),
            }
        } else {
            for _ in idx..p.events.len() {
                p.rollback();
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct CompletedMarker {
    pos: u32,
    // end_pos: u32,
    kind: SyntaxKind,
}

impl CompletedMarker {
    fn new(pos: u32 /*, end_pos: u32*/, kind: SyntaxKind) -> Self {
        CompletedMarker { pos /*, end_pos*/, kind }
    }

    /// This method allows to create a new node which starts
    /// *before* the current one. That is, parser could start
    /// node `A`, then complete it, and then after parsing the
    /// whole `A`, decide that it should have started some node
    /// `B` before starting `A`. `precede` allows to do exactly
    /// that. See also docs about
    /// [`Event::Start::forward_parent`](crate::event::Event::Start::forward_parent).
    ///
    /// Given completed events `[START, FINISH]` and its corresponding
    /// `CompletedMarker(pos: 0, _)`.
    /// Append a new `START` events as `[START, FINISH, NEWSTART]`,
    /// then mark `NEWSTART` as `START`'s parent with saving its relative
    /// distance to `NEWSTART` into forward_parent(=2 in this case);
    pub(crate) fn precede(self, p: &mut Parser) -> Marker {
        let new_pos = p.start();
        let idx = self.pos as usize;
        match &mut p.events[idx] {
            Event::Start { forward_parent, .. } => {
                *forward_parent = Some(new_pos.pos - self.pos);
            }
            _ => unreachable!(),
        }
        new_pos
    }

    /// Extends this completed marker *to the left* up to `m`.
    pub(crate) fn extend_to(self, p: &mut Parser, mut m: Marker) -> CompletedMarker {
        m.bomb.defuse();
        let idx = m.pos as usize;
        match &mut p.events[idx] {
            Event::Start { forward_parent, .. } => {
                *forward_parent = Some(self.pos - m.pos);
            }
            _ => unreachable!(),
        }
        self
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.kind
    }

    /// Abandons the syntax tree node. All its children events are dropped and position restored.
    pub(crate) fn abandon_with_rollback(self, p: &mut Parser) {
        let idx = self.pos as usize;
        if idx == p.events.len() - 1 {
            match p.events.pop() {
                Some(Event::Start {
                    kind: TOMBSTONE,
                    forward_parent: None,
                }) => (),
                _ => unreachable!(),
            }
        } else {
            for _ in idx..p.events.len() {
                p.rollback();
            }
        }
    }

    // pub(crate) fn last_token(&self, p: &Parser<'_>) -> Option<SyntaxKind> {
    //     let end_pos = self.end_pos as usize;
    //     // debug_assert_eq!(p.events[end_pos - 1], Event::Finish);
    //     p.events[..end_pos].iter().rev().find_map(|event| match event {
    //         Event::Token { kind, .. } => Some(*kind),
    //         _ => None,
    //     })
    // }
}
