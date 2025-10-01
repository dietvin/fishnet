use uuid::Uuid;

use crate::core::{filter::ChunkInfo, reformater::reformated::ReformatedData};

pub(crate) struct OutputData {
    read_id: Uuid,
    start_index_on_alignment: usize,
    matched_region_name: String,
    reformated_data: ReformatedData
}

impl OutputData {
    pub(crate) fn new(
        read_id: Uuid,
        chunk_info: ChunkInfo,
        reformated_data: ReformatedData
    ) -> Self {
        Self { 
            read_id, 
            start_index_on_alignment: chunk_info.start_index, 
            matched_region_name: chunk_info.matched_element_name.clone(), 
            reformated_data 
        }
    }

    pub(crate) fn into_inner(self) -> (Uuid, usize, String, ReformatedData) {
        (
            self.read_id,
            self.start_index_on_alignment,
            self.matched_region_name,
            self.reformated_data
        )
    }
}