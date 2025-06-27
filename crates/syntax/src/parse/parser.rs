//! See [`Parser`].

use crate::parse::event::Event;
use crate::parse::recovery_set::{RecoverySet, RecoveryToken};
use crate::parse::text_token_source::TextTokenSource;
use crate::parse::token_set::TokenSet;
use crate::parse::ParseError;
use crate::{
    SyntaxKind::{self, EOF, ERROR, TOMBSTONE},
    T,
};
use drop_bomb::DropBomb;
use std::ops::ControlFlow;

/// `Parser` struct provides the low-level API for
/// navigating through the stream of tokens and
/// constructing the parse tree. The actual parsing
/// happens in the [`grammar`](super::grammar) module.
///
/// However, the result of this `Parser` is not a real
/// tree, but rather a flat stream of events of the form
/// "start expression, consume number literal,
/// finish expression". See `Event` docs for more.
pub struct Parser {
    pub(crate) token_source: TextTokenSource,
    events: Vec<Event>,
    recovery_set_stack: Vec<RecoverySet>,
}

impl Parser {
    pub(super) fn new(token_source: TextTokenSource) -> Parser {
        Parser {
            token_source,
            events: vec![],
            // recovery_tokens: vec![],
            recovery_set_stack: vec![],
        }
    }

    pub(crate) fn finish(self) -> Vec<Event> {
        self.events
    }

    pub(crate) fn pos(&self) -> usize {
        self.token_source.current_pos()
    }

    pub(crate) fn should_stop(&self, stop_at: &RecoverySet) -> bool {
        stop_at.contains(self.current())
    }

    pub(crate) fn at_same_pos_as(&self, last_pos: Option<usize>) -> bool {
        last_pos.is_some_and(|it| it == self.pos())
    }

    #[allow(non_snake_case)]
    pub(crate) fn iterate_to_EOF(
        &mut self,
        stop_at: impl Into<TokenSet>,
        f: impl FnMut(&mut Parser) -> ControlFlow<()>,
    ) {
        self.iterate_to_EOF_rec(stop_at.into(), f)
    }

    #[allow(non_snake_case)]
    pub(crate) fn iterate_to_EOF_rec(
        &mut self,
        stop_at_rec: impl Into<RecoverySet>,
        mut f: impl FnMut(&mut Parser) -> ControlFlow<()>,
    ) {
        let stop_at = stop_at_rec.into();
        while !self.at(EOF) && !self.should_stop(&stop_at) {
            let pos_before = self.pos();

            // add loop end to recovery set
            self.recovery_set_stack
                .push(self.outer_recovery_set().with_merged(stop_at.clone()));
            let cf = f(self);
            self.recovery_set_stack.pop();

            if matches!(cf, ControlFlow::Break(_)) {
                break;
            }
            let outer_rec = self.outer_recovery_set();
            if self.should_stop(&outer_rec) {
                break;
            }
            if self.pos() == pos_before {
                // iteration is stuck
                #[cfg(debug_assertions)]
                panic!("iteration is stuck at {:?}", self.current_context());
                break;
            }
        }
    }

    pub(crate) fn event_pos(&self) -> usize {
        self.events.len() - 1
    }

    /// Returns the kind of the current token.
    /// If parser has already reached the end of input,
    /// the special `EOF` kind is returned.
    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    pub(crate) fn current_context(&self) -> (&str, &str, &str) {
        (
            self.token_source.prev_text(),
            self.token_source.current_text(),
            self.token_source.next_text(),
        )
    }

    pub(crate) fn current_text(&self) -> &str {
        self.token_source.current_text()
    }

    /// How much whitespaces are skipped from prev to curr
    pub(crate) fn prev_ws_at(&self, n: usize) -> usize {
        let nth = self.token_source.curr_pos() + n;
        if nth == 0 {
            return 0;
        }
        let Some(from_range) = self.token_source.token_range(nth - 1) else {
            return 0;
        };
        let Some(to_range) = self.token_source.token_range(nth) else {
            return 0;
        };
        (to_range.start() - from_range.end()).into()
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
        self.push_error(message);
    }

    pub(crate) fn push_error(&mut self, message: impl Into<String>) {
        let msg = ParseError(Box::new(message.into()));
        self.push_event(Event::Error { msg });
    }

    /// Consume the next token if it is `kind` or emit an error
    /// otherwise.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        self.expect_with_error(kind, &format!("expected {:?}", kind))
    }

    /// Consume the next token if it is `kind` or emit an error
    /// otherwise.
    pub(crate) fn expect_with_error(&mut self, kind: SyntaxKind, error_message: &str) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.push_error(error_message);
        false
    }

    /// adds error and then bumps until `stop()` is true
    pub(crate) fn error_and_recover(&mut self, message: &str, rs: impl Into<RecoverySet>) {
        // if the next token is stop token, just push error,
        // otherwise wrap the next token with the error node and start `recover_until()`
        let rec_set = self.outer_recovery_set().with_merged(rs.into());
        if rec_set.contains_current(self) {
            self.push_error(message);
            return;
        }
        self.error_and_bump(message);
        // bump tokens until reached `stop_token`
        while !self.at(EOF) && !rec_set.contains_current(self) {
            self.bump_any();
        }
    }

    pub(crate) fn error_and_bump(&mut self, message: &str) {
        let m = self.start();
        self.bump_any();
        self.push_error(message);
        m.complete(self, ERROR);
    }

    fn do_bump(&mut self, kind: SyntaxKind, n_raw_tokens: u8) {
        for _ in 0..n_raw_tokens {
            self.token_source.bump();
        }

        self.push_event(Event::Token { kind, n_raw_tokens });
    }

    fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    fn pop_event(&mut self) -> Option<Event> {
        let event = self.events.pop();
        if let Some(Event::Token { kind: _, n_raw_tokens }) = &event {
            for _ in 0..*n_raw_tokens {
                self.token_source.pop_position();
            }
        }
        event
    }

    // /// add event before the last token (used for errors)
    // fn push_event_preceding(&mut self, event: Event) {
    //     assert!(!self.events.is_empty());
    //     self.events.insert(self.events.len() - 1, event);
    // }
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
        // replace TOMBSTONE with `kind`
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
                p.pop_event();
            }
        }
    }
}

// recovery sets
impl Parser {
    pub fn outer_recovery_set(&self) -> RecoverySet {
        self.recovery_set_stack
            .last()
            .cloned()
            .unwrap_or(RecoverySet::new())
    }

    pub(crate) fn with_recovery_set<T>(
        &mut self,
        recovery_set: RecoverySet,
        f: impl FnOnce(&mut Parser) -> T,
    ) -> T {
        self.recovery_set_stack
            .push(self.outer_recovery_set().with_merged(recovery_set));
        let res = f(self);
        self.recovery_set_stack.pop();
        res
    }

    pub(crate) fn reset_recovery_set<T>(&mut self, f: impl FnOnce(&mut Parser) -> T) -> T {
        self.recovery_set_stack.push(RecoverySet::new());
        // self.recovery_set_stack
        //     .push(self.outer_recovery_set().with_merged(recovery_set));
        let res = f(self);
        self.recovery_set_stack.pop();
        res
    }

    pub(crate) fn with_recovery_token_set<T>(
        &mut self,
        token_set: impl Into<TokenSet>,
        f: impl FnOnce(&mut Parser) -> T,
    ) -> T {
        self.recovery_set_stack
            .push(self.outer_recovery_set().with_token_set(token_set));
        let res = f(self);
        self.recovery_set_stack.pop();
        res
    }

    pub(crate) fn with_recovery_tokens<T>(
        &mut self,
        tokens: Vec<RecoveryToken>,
        f: impl FnOnce(&mut Parser) -> T,
    ) -> T {
        let mut new_rec_set = self.outer_recovery_set();
        for token in tokens.clone() {
            new_rec_set = new_rec_set.with_recovery_token(token);
        }
        self.recovery_set_stack.push(new_rec_set);
        let res = f(self);
        self.recovery_set_stack.pop();
        res
    }

    pub(crate) fn with_recovery_token<'t, T>(
        &mut self,
        token: impl Into<RecoveryToken>,
        f: impl FnOnce(&mut Parser) -> T,
    ) -> T {
        self.with_recovery_tokens(vec![token.into()], f)
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
    pub(crate) fn abandon_with_rollback(self, p: &mut Parser, parent_event_pos: usize) {
        while p.events.len() != parent_event_pos + 1 {
            p.pop_event();
        }
    }

    // pub(crate) fn last_token(&self, p: &Parser) -> Option<SyntaxKind> {
    //     let end_pos = self.end_pos as usize;
    //     // debug_assert_eq!(p.events[end_pos - 1], Event::Finish);
    //     p.events[..end_pos].iter().rev().find_map(|event| match event {
    //         Event::Token { kind, .. } => Some(*kind),
    //         _ => None,
    //     })
    // }
}
