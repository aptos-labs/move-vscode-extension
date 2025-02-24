use crate::nameres::address::Address;
use crate::nameres::paths::PathResolutionContext;
use crate::nameres::processors::{ProcessingStatus, Processor};

pub fn process_module_path_resolve_variants(
    ctx: PathResolutionContext,
    address: Address,
    processor: &impl Processor,
) -> ProcessingStatus {
    // get all modules
    // filter by the address
    // process

    ProcessingStatus::Continue
}
