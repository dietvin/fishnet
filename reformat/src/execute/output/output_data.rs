use uuid::Uuid;

use crate::core::{filter::MatchedFilterInfo, reformater::reformated::ReformatedData};

/// Holds constant metadata and variable processed data
/// 
/// Constant metadata is present with all processing approaches. 
/// These fields are:
/// * `read_id`
/// * `start_index_on_alignment`
/// * `matched_region_name`
/// 
/// The variable processed data changes depending on the 
/// processing approach that was used. To account for both
/// options, the data is wrapped in the [`ReformatedData`]
/// enum.
pub(crate) struct OutputData {
    read_id: Uuid,
    start_index_on_read: usize,
    matched_region_name: String,
    reformated_data: ReformatedData
}

impl OutputData {
    /// Initializes a new OutputData instance from a processed
    /// read.
    /// 
    /// # Arguments
    /// * `read_id` - The id of the reformated read
    /// * `chunk_info` - Infos about the matching reference region
    ///                  or motif
    /// * `reformated_data` - The reformated data itself
    pub(crate) fn new(
        read_id: Uuid,
        chunk_info: MatchedFilterInfo,
        reformated_data: ReformatedData
    ) -> Self {
        Self { 
            read_id, 
            start_index_on_read: chunk_info.start_index, 
            matched_region_name: chunk_info.matched_element_name.clone(), 
            reformated_data 
        }
    }

    /// Consumes self and returns the contained elements.
    /// 
    /// # Returns
    /// A tuple containing the following data:
    /// 1. The read ID
    /// 2. The start index of the region of interest on the read
    /// 3. The name of the region of interest
    /// 4. The reformated data 
    pub(crate) fn into_inner(self) -> (Uuid, usize, String, ReformatedData) {
        (
            self.read_id,
            self.start_index_on_read,
            self.matched_region_name,
            self.reformated_data
        )
    }
}