# Pod5Read
The `Pod5Read` struct grants access to the signal and metadata corresponding to a single read, all of which can be accessed by various getter functions. A read can not be manually initialized, but needs to be retrieved from a pod5 file or dataset.

The following table shows all fields that are accessible from a Pod5Read. The descriptions are taken mostly from the [official pod5 docs](https://github.com/nanoporetech/pod5-file-format/blob/master/docs/tables/reads.toml).
| Field | Description |
|-|-|
| read_id | Globally-unique identifier for the read, can be converted to a string form (using standard routines in other libraries) which matches how reads are identified elsewhere |
| signal | The actual signal |
| signal_indices | A list of zero-indexed row numbers in the Signal table. This must be all the rows in the Signal table that have a matching read_id, in order. It functions as an index for the Signal table |
| read_number | The read number on channel. This is increasing but typically not necessarily consecutive |
| start | How many samples were taken on this channel before the read started (since the data acquisition period began). This can be combined with the sample rate to get a time in seconds for the start of the read relative to the start of data acquisition |
| median_before | "The level of current in the well before this read (typically the open pore level of the well). If the level is not known (eg: due to a mux change), this should be nulled out |
| num_minknow_events | Number of minknow events that the read contains |
| tracked_scaling | Collects tracked_scaling_shift (Shift for tracked read scaling values (based on previous reads shift)) and tracked_scaling_scale (Scale for tracked read scaling values (based on previous reads shift)) |
| predicted_scaling | Collects predicted_scaling_shift (Shift for predicted read scaling values (based on this read's raw signal)) and predicted_scaling_scale (Scale for predicted read scaling values (based on this read's raw signal)) |
| num_reads_since_mux_change | Number of selected reads since the last mux change on this reads channel |
| time_since_mux_change | Time in seconds since the last mux change on this reads channel |
| num_samples | The full length of the signal for this read in samples (equal to the sum of all 'samples' fields of signal chunks) |
| pore | Collects the pore_type (Name of the pore type present in the well), channel (1-indexed channel) and well (1-indexed well (typically 1, 2, 3 or 4)) information |
| calibration | Collects calibration_offset (Calibration offset used to scale raw ADC data into pA readings) and calibration_scale (Calibration scale factor used to scale raw ADC data into pA readings) |
| end_reason | Collects end_reason (The end reason, currently one of: unknown, mux_change, unblock_mux_change, data_service_unblock_mux_change, signal_positive, signal_negative, api_request, device_data_error, analysis_config_change or paused) and end_reason_forced (True if this read was ended 'forcibly' (eg: mux_change, unblock), false if it was a data-driven read break (signal_positive, signal_negative). This allows simple categorisation even in the presence of new reasons that reading code is unaware of) info |
| run_info_id | Id that matches the acquisition_id in the run info stored for the overarching Pod5File | 

The user has the choice to call the standard getter function (e.g. `Pod5Read::read_id()`), which wraps the value in an option, or the *require* function (e.g. `Pod5Read::require_read_id()`) which wraps the value in a Result. All information should be present for a standard read, but the internal arrow schema lists the fields for the data as optional, so they can technically be missing. That's why the values are not directly available.