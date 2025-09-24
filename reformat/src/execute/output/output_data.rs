pub(crate) struct OutputRow {}

pub(crate) enum OutputData {
    Stats { data: StatsOutput },
    Interpolate { data: InterpolateOutput }
}

pub(crate) struct StatsOutput {}

pub(crate) struct InterpolateOutput {}