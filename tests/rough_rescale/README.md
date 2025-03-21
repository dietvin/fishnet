# Integration tests for the rough rescaling functions

To test if the implemented functions behave as expected (i.e. the same way as Remora does it), I extracted the input data and the returned shift and scale values from the `remora.refine_signal_map.SigMapRefiner.rough_rescale` function.

With this I can give the input data to the Rust function and test if the calculated shift and scale values match the ones from Remora. 

## Parameter combinations

In the Remora implementation three parameters are given: 
- the rescaling algorithm ("least_squares" / "theil_sen") (`sig_map_refiner.rough_rescale_method`)
- the number of bases to clip from the start & end (`clip_bases`)
- whether to use the center base only (`use_base_center`) 

I used the following parameters for testing:
- sig_map_refiner.rough_rescale_method: `least_squares` / `theil_sen`
- clip_bases: `0` / `10`
- use_base_center: `True` / `False`

This resulted in the following parameter combinations:
- `least_squares` + `0` + `True`
- `least_squares` + `0` + `False`
- `least_squares` + `10` + `True`
- `least_squares` + `10` + `False` (*)
- `theil_sen` + `0` + `True`
- `theil_sen` + `0` + `False`
- `theil_sen` + `10` + `True`
- `theil_sen` + `10` + `False` (*)

Two combinations were ignored. This is due to the fact that the Rust implementation has slight differences in their implementation. 

First, in Remora base clipping is only performed if use_base_center is true. In the Rust implementation the base clipping is performed regardless of the use_base_center value. Accordingly two parameter combinations **(*)** are not tested, as here we can expect different results. I kept clip_bases=0 & use_base_center=True because no clipping is performed here in Remora anyway, so the results are expected to be the same in Remora and Rust.

## Data extraction

To extract the data a decorator function was added in remora.refine_signal_map: 

```python
import functools
import json
import inspect
import os
import numpy as np
import datetime
def export_function_io(output_dir, attrs_to_log=None):
    """Decorator to log function inputs, instance attributes, and outputs, including default arguments."""
    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            obj = args[0] if args and hasattr(args[0], "__dict__") else None  # Detect `self`
            func_args = args[1:] if obj else args  # Remove `self` if it's a method

            # Get function signature and default values
            sig = inspect.signature(func)
            bound_args = sig.bind_partial(*args, **kwargs)
            bound_args.apply_defaults()  # Apply default values

            # Convert arguments to a log-friendly format
            def format_arg(value):
                return " ".join([str(v) for v in value]) if isinstance(value, np.ndarray) else value

            data = {
                "function": func.__name__,
                "args": {key: format_arg(value) for key, value in bound_args.arguments.items()},
                "result": None  # Placeholder for now
            }

            # Capture class attributes if it's a method
            if obj and attrs_to_log:
                data["class_attrs"] = {attr: getattr(obj, attr, None) for attr in attrs_to_log}

            # Call function and store the result
            result = func(*args, **kwargs)
            data["result"] = format_arg(result)

            # Ensure output directory exists
            os.makedirs(output_dir, exist_ok=True)
            time = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
            log_path = os.path.join(output_dir, f"{func.__name__}_{time}.json")

            # Write to JSON file
            with open(log_path, "w") as f:
                f.write(json.dumps(data, default=str) + "\n")

            return result
        return wrapper
    return decorator
```

The decorator was applied to the rough_rescale function:
```python
@export_function_io(<OUTPUT-DIRECTORY>, attrs_to_log=["rough_rescale_method"])
    def rough_rescale(...):
        ...
```

For each combination the parameters were adjusted by hand in `remora.data_chunks.RemoraRead.refine_signal_mapping`:
```python
...
if sig_map_refiner.do_rough_rescale:
    prev_shift, prev_scale = self.shift, self.scale
    sig_map_refiner.rough_rescale_method = <VALUE>
    self.shift, self.scale = sig_map_refiner.rough_rescale(
        self.shift,
        self.scale,
        self.seq_to_sig_map,
        self.int_seq,
        self.dacs,
        clip_bases=<VALUE>,
        use_base_center=<VALUE>            
    )
...
```

Then the package was installed into a test environment via: `pip install -e <...>/remora`

Then remora was executed from a jupyter notebook:
```python
import pod5
from remora import io, refine_signal_map
from pathlib import Path
import numpy as np
test_data_root = Path(<PATH-TO-EXAMPLE-DATA>)
pod5_dr = pod5.DatasetReader(test_data_root)
bam_fh = io.ReadIndexedBam(test_data_root / "can_mappings.bam")

for read_id in bam_fh.read_ids: 
    pod5_read = pod5_dr.get_read(read_id)
    bam_read = bam_fh.get_first_alignment(read_id)

    read = io.Read.from_pod5_and_alignment(pod5_read, bam_read)
    query_to_signal_initial = read.query_to_signal
    ref_to_signal_initial = read.ref_to_signal

    level_table = test_data_root / "levels.txt"
    sig_map_refiner = refine_signal_map.SigMapRefiner(
        kmer_model_filename=level_table,
        do_rough_rescale=True,
        scale_iters=0,
        do_fix_guage=True,
    )

    read.set_refine_signal_mapping(sig_map_refiner)

    query_to_signal_refined = read.query_to_signal
    ref_to_signal_refined = read.ref_to_signal
```

After each execution a JSON file was created containing the following information:
```json
{
    "function": "rough_rescale",
    "args": {
        "self": "Loaded 9-mer table with 7 central position. Rough re-scaling will be executed. Signal mapping refinement will be executed using the dwell_penalty refinement method (band half width: 5). Short dwell penalty array set to [8.  4.5 2. ].",
        "shift": 887.9108857812896,
        "scale": 180.72997974885317,
        "seq_to_sig_map": "0 45 50 70 ... 83225 83234",
        "int_seq": "2 3 2 ... 0 2 1 0 3",
        "dacs": "987 990 1011 989 999 990 987 ... 945 934 943 932 913 929 919 911 774 747 754 754 779 745 733",
        "quants": "0.05 0.1 0.15000000000000002 0.2 0.25 0.3 0.35000000000000003 0.4 0.45 0.5 0.55 0.6000000000000001 0.6500000000000001 0.7000000000000001 0.7500000000000001 0.8 0.8500000000000001 0.9000000000000001 0.9500000000000001",
        "clip_bases": 0,
        "use_base_center": false
    },
    "result": [
        881.2671313105108,
        197.4755979507768
    ],
    "class_attrs": {
        "rough_rescale_method": "least_squares"
    }
}
``` 