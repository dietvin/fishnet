pub struct RegRegion {
    ref_name: String,
    ref_start: usize,
    ref_end: usize
}

impl RegRegion {
    pub fn from_bed(ref_name: &str, ref_start: usize, ref_end: usize) -> Self {
        todo!()
    }

    pub fn from_center_pos(ref_name: &str, center_pos: usize, half_size: usize) -> Self {
        todo!()
    }
}