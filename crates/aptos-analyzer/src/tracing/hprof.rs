//! Consumer of `tracing` data, which prints a hierarchical profile.
//!
//! Based on https://github.com/davidbarsky/tracing-tree, but does less, while
//! actually printing timings for spans by default.
//!
//! Usage:
//!
//! ```ignore
//! tracing_span_tree::span_tree()
//!     .aggregate(true)
//!     .enable();
//! ```
//!
//! Example output:
//!
//! ```text
//! 8.37ms           top_level
//!   1.09ms           middle
//!     1.06ms           leaf
//!   1.06ms           middle
//!   3.12ms           middle
//!     1.06ms           leaf
//!   3.06ms           middle
//! ```
//!
//! Same data, but with `.aggregate(true)`:
//!
//! ```text
//! 8.39ms           top_level
//!  8.35ms    4      middle
//!    2.13ms    2      leaf
//! ```

use duration_string::DurationString;
use std::{
    env, fmt, mem,
    time::{Duration, Instant},
};
use tracing::{
    Event, Id, Subscriber, debug,
    field::{Field, Visit},
    span::Attributes,
};
use tracing_subscriber::{
    Layer,
    layer::Context,
    prelude::*,
    registry::{LookupSpan, Registry},
};

pub fn span_tree() -> SpanTree {
    SpanTree::default()
}

#[derive(Default)]
pub struct SpanTree {
    aggregate: bool,
}

impl SpanTree {
    /// Merge identical sibling spans together.
    pub fn aggregate(self, yes: bool) -> SpanTree {
        SpanTree { aggregate: yes, ..self }
    }
    /// Set as a global subscriber
    pub fn enable(self) {
        let subscriber = Registry::default().with(self);
        tracing::subscriber::set_global_default(subscriber)
            .unwrap_or_else(|_| debug!("Global subscriber is already set"));
    }
}

struct Data {
    start: Instant,
    children: Vec<Node>,
}

impl Data {
    fn new(attrs: &Attributes<'_>) -> Self {
        let mut span = Self {
            start: Instant::now(),
            children: Vec::new(),
        };
        attrs.record(&mut span);
        span
    }
    fn into_node(self, name: &'static str) -> Node {
        Node {
            name,
            count: 1,
            duration: self.start.elapsed(),
            children: self.children,
        }
    }
}

impl Visit for Data {
    fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {}
}

impl<S> Layer<S> for SpanTree
where
    S: Subscriber + for<'span> LookupSpan<'span> + fmt::Debug,
{
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<S>) {
        let span = ctx.span(id).unwrap();

        let data = Data::new(attrs);
        span.extensions_mut().insert(data);
    }

    fn on_event(&self, _event: &Event<'_>, _ctx: Context<S>) {}

    fn on_close(&self, id: Id, ctx: Context<S>) {
        let span = ctx.span(&id).unwrap();
        let data = span.extensions_mut().remove::<Data>().unwrap();
        let mut node = data.into_node(span.name());

        match span.parent() {
            Some(parent_span) => {
                parent_span
                    .extensions_mut()
                    .get_mut::<Data>()
                    .unwrap()
                    .children
                    .push(node);
            }
            None => {
                if self.aggregate {
                    node.aggregate()
                }
                node.print()
            }
        }
    }
}

#[derive(Default)]
struct Node {
    name: &'static str,
    count: u32,
    duration: Duration,
    children: Vec<Node>,
}

impl Node {
    fn print(&self) {
        self.go(0)
    }
    fn go(&self, level: usize) {
        // filter out everything less than 1ms
        if let Ok(min_duration) = env::var("APT_PROFILER_MIN_DURATION") {
            let min_duration: duration_string::Result<Duration> =
                DurationString::from_string(min_duration).map(|it| it.into());
            match min_duration {
                Ok(min_duration) => {
                    if self.duration < min_duration {
                        return;
                    }
                }
                Err(err) => {
                    tracing::error!("invalid value for APT_PROFILER_MIN_DURATION: {}", err);
                }
            }
        }

        let duration = format!("{:3.2?}", self.duration);
        let count = if self.count > 1 {
            self.count.to_string()
        } else {
            String::new()
        };
        eprintln!(
            "{:width$}  {:<9} {:<6} {}",
            "",
            duration,
            count,
            self.name,
            width = level * 2
        );
        for child in &self.children {
            child.go(level + 1)
        }
        if level == 0 {
            eprintln!()
        }
    }

    fn aggregate(&mut self) {
        if self.children.is_empty() {
            return;
        }

        self.children.sort_by_key(|it| it.name);
        let mut idx = 0;
        for i in 1..self.children.len() {
            if self.children[idx].name == self.children[i].name {
                let child = mem::take(&mut self.children[i]);
                self.children[idx].duration += child.duration;
                self.children[idx].count += child.count;
                self.children[idx].children.extend(child.children);
            } else {
                idx += 1;
                assert!(idx <= i);
                self.children.swap(idx, i);
            }
        }
        self.children.truncate(idx + 1);
        for child in &mut self.children {
            child.aggregate()
        }
    }
}
