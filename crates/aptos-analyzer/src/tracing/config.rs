//! Simple logger that logs either to stderr or to a file, using `tracing_subscriber`
//! filter syntax and `tracing_appender` for non blocking output.

use crate::tracing::json;
use anyhow::Context;
use std::env;
use std::str::FromStr;
use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    Layer, Registry,
    filter::{Targets, filter_fn},
    fmt::{MakeWriter, time},
    layer::SubscriberExt,
};
use tracing_tree::HierarchicalLayer;

#[derive(Debug)]
pub struct Config<T> {
    pub writer: T,
    pub filter: String,

    /// Filtering syntax, set in a shell:
    /// ```text
    /// env RA_PROFILE_JSON=foo|bar|baz
    /// ```
    pub json_profile_filter: Option<String>,
}

impl<T> Config<T>
where
    T: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    pub fn init(self) -> anyhow::Result<()> {
        let targets_filter: Targets = self
            .filter
            .parse()
            .with_context(|| format!("invalid log filter: `{}`", self.filter))?;

        let writer = self.writer;

        let ra_fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_ansi(false)
            .with_writer(writer);

        let ra_fmt_layer = match time::OffsetTime::local_rfc_3339() {
            Ok(timer) => {
                // If we can get the time offset, format logs with the timezone.
                ra_fmt_layer.with_timer(timer).boxed()
            }
            Err(_) => {
                // Use system time if we can't get the time offset. This should
                // never happen on Linux, but can happen on e.g. OpenBSD.
                ra_fmt_layer.boxed()
            }
        }
        .with_filter(targets_filter);

        // let chalk_layer = match self.chalk_filter {
        //     Some(chalk_filter) => {
        //         let level: LevelFilter =
        //             chalk_filter.parse().with_context(|| "invalid chalk log filter")?;
        //
        //         let chalk_filter = Targets::new()
        //             .with_target("chalk_solve", level)
        //             .with_target("chalk_ir", level)
        //             .with_target("chalk_recursive", level);
        //         // TODO: remove `.with_filter(LevelFilter::OFF)` on the `None` branch.
        //         HierarchicalLayer::default()
        //             .with_indent_lines(true)
        //             .with_ansi(false)
        //             .with_indent_amount(2)
        //             .with_writer(io::stderr)
        //             .with_filter(chalk_filter)
        //             .boxed()
        //     }
        //     None => None::<HierarchicalLayer>.with_filter(LevelFilter::OFF).boxed(),
        // };

        let json_profiler_layer = match self.json_profile_filter {
            Some(spec) => {
                let filter = json::JsonFilter::from_spec(&spec);
                let filter = filter_fn(move |metadata| {
                    let allowed = match &filter.allowed_names {
                        Some(names) => names.contains(metadata.name()),
                        None => true,
                    };

                    allowed && metadata.is_span()
                });
                Some(json::TimingLayer::new(std::io::stderr).with_filter(filter))
            }
            None => None,
        };

        let level = Level::from_str(&env::var("RA_LOG").ok().unwrap_or_else(|| "error".to_owned()))?;
        let subscriber = Registry::default()
            .with(
                HierarchicalLayer::new(2)
                    .with_indent_lines(true)
                    .with_deferred_spans(true)
                    .with_filter(LevelFilter::from_level(level)),
            )
            .with(ra_fmt_layer)
            .with(json_profiler_layer);

        tracing::subscriber::set_global_default(subscriber)?;

        Ok(())
    }
}
