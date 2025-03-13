use crate::nameres::namespaces::{NsSet, NsSetExt};
use crate::nameres::scope::ScopeEntry;
use std::cell::RefCell;
use syntax::ast;
use syntax::ast::HasName;

pub enum ProcessingStatus {
    Stop,
    Continue,
}

impl ProcessingStatus {
    pub fn is_stop(&self) -> bool {
        matches!(self, ProcessingStatus::Stop)
    }
    pub fn is_continue(&self) -> bool {
        !self.is_stop()
    }
}

pub trait Processor {
    fn process(&self, entry: ScopeEntry) -> ProcessingStatus;

    fn process_named(&self, item: impl ast::HasName, ns: NsSet) -> ProcessingStatus {
        ProcessingStatus::Continue
        // match ScopeEntry::from_named(item, ns) {
        //     None => ProcessingStatus::Continue,
        //     Some(entry) => self.process(entry),
        // }
    }

    fn process_all_named<NamedItem: HasName>(
        &self,
        named_items: impl IntoIterator<Item = NamedItem>,
        ns: NsSet,
    ) -> ProcessingStatus {
        for named_item in named_items {
            let status = self.process_named(named_item, ns.clone());
            if status.is_stop() {
                return ProcessingStatus::Stop;
            }
        }
        ProcessingStatus::Continue
    }
}

pub fn collect_entries<F>(f: F) -> Vec<ScopeEntry>
where
    F: FnOnce(&ScopeEntriesCollector) -> (),
{
    let collector = ScopeEntriesCollector::new();
    f(&collector);
    collector.into_result()
}

pub struct ScopeEntriesCollector {
    entries: RefCell<Vec<ScopeEntry>>,
}

impl ScopeEntriesCollector {
    pub fn new() -> Self {
        ScopeEntriesCollector {
            entries: RefCell::new(vec![]),
        }
    }

    pub fn into_result(self) -> Vec<ScopeEntry> {
        self.entries.into_inner()
    }
}

impl Processor for ScopeEntriesCollector {
    fn process(&self, entry: ScopeEntry) -> ProcessingStatus {
        self.entries.borrow_mut().push(entry);
        ProcessingStatus::Continue
    }
}

pub fn collect_entries_with_ref_name<F>(ref_name: String, f: F) -> Vec<ScopeEntry>
where
    F: FnOnce(&ReferenceNameScopeEntriesCollector) -> (),
{
    let collector = ReferenceNameScopeEntriesCollector::new(ref_name);
    f(&collector);
    collector.result.into_inner()
}

pub struct ReferenceNameScopeEntriesCollector {
    ref_name: String,
    pub result: RefCell<Vec<ScopeEntry>>,
}

impl ReferenceNameScopeEntriesCollector {
    pub fn new(ref_name: String) -> Self {
        ReferenceNameScopeEntriesCollector {
            ref_name,
            result: RefCell::new(vec![]),
        }
    }
}

impl Processor for ReferenceNameScopeEntriesCollector {
    fn process(&self, entry: ScopeEntry) -> ProcessingStatus {
        if entry.name.as_str() == self.ref_name {
            self.result.borrow_mut().push(entry);
            // todo: remove it to support multiple resolutions
            return ProcessingStatus::Stop;
        }
        ProcessingStatus::Continue
    }
}

pub fn filter_ns_processor<P: Processor>(ns: NsSet, processor: &P) -> FilterNsProcessor<P> {
    FilterNsProcessor::new(ns, processor)
}

pub struct FilterNsProcessor<'a, P: Processor> {
    ns: NsSet,
    original_processor: &'a P,
}

impl<'a, P: Processor> FilterNsProcessor<'a, P> {
    pub fn new(ns: NsSet, processor: &'a P) -> Self {
        FilterNsProcessor {
            ns,
            original_processor: processor,
        }
    }
}

impl<'a, P: Processor> Processor for FilterNsProcessor<'a, P> {
    fn process(&self, entry: ScopeEntry) -> ProcessingStatus {
        if self.ns.contains_any_of(entry.ns) {
            return self.original_processor.process(entry);
        }
        ProcessingStatus::Continue
    }
}

// pub struct ShadowingProcessor {
//     encountered: RefCell<HashMap<Name, NsSet>>,
// }

// impl Processor for ShadowingProcessor {
//     fn process(&self, entry: ScopeEntry) -> Option<()> {
//         match self.encountered.borrow_mut().entry(entry.name) {
//             Entry::Occupied(_) => {}
//             Entry::Vacant(e) => {
//                 e.insert(entry.ns);
//                 // continue
//                 None
//             }
//         }
//     }
// }
