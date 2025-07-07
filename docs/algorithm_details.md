# Algorithm details

## Table of contents
- [Initial alignment](#initial-alignment)
- [Refinement](#refinement)
    - [Refinement workflow](#refinement-workflow)
    - [Signal normalization](#signal-normalization)
    - [Rough rescaling](#rough-rescaling)
    - [Boundary optimization](#boundary-optimization)
    - [Iterative refinement strategy](#iterative-refinement-strategy)

## Initial alignment
An itital alignment is constructed using the move table generated during base-calling. This is an array of boolean values that indicates when the sequencer detected a new base in the signal, represented by a 1. By combining this information with the sampling stride, a mapping from positions in the base-called (query) sequence to positions in the raw signal is created. 

If the read is mapped to a reference, the associated CIGAR string can be used to derive a reference-to-signal alignment. This is done by first computing a reference-to-query mapping based on the CIGAR operations and then translating it to signal coordinates via the query-to-signal mapping, followed by linear interpolation to obtain a dense signal alignment for each reference position.


## Refinement
The inital alignment is solely based on the move table and does not account for the actual measured signal intensities. The refinement process improves this alignment by shifting alignment boundaries in a way that minimizes the discrepancy between observed signal levels and the expected signal levels for the corresponding k-mers, as defined by ONT's k-mer models.

The refinement is performed iteratively with configurable parameters that control the number of refinement iterations, the algorithms used for the refinement and the algorithm used for signal normalization.

### Refinement workflow
The refinement process follows this general workflow:
1. **Initial signal normalization**: Raw signal measurements are transformed to normalized values using calibration parameters from the sequencer and signal statistics (see  [Signal normalization](#signal-normalization))

2. **Optional rough rescaling**: A coarse-grained scaling adjustment using quantile-based regression to improve the baseline divergence between observed and expected measurements (see  [Rough rescaling](#rough-rescaling))

3. **Iterative refinement**: Multiple rounds of boundary optimization with recalibration between iterations (see  [Boundary optimization](#boundary-optimization))

4. **Final alignment**: Alignment with boundaries shifted to minimize the distance between observed and expected signal measurements


### Signal normalization
Singal normalization transforms raw signal measurements into normalized values using a two-step process:

1. **Initial scaling calculations**: Raw measurements are converted to picoampere (pA) units and are then normalized using calibration parameteres stored in the pod5 file:

    $ signal\_norm_i = \frac{signal_i-shift_{init}}{scale_{init}} $

2. **Iterative recalibration**: After each boundary optimization step, the normalization parameters are recalibrated using regression analysis between observed and expected signal levels. Afterwards the signal is normalized on these updated parameters:

    $ signal\_norm_i = \frac{signal_i-shift_{recal}}{scale_{recal}} $

The regression can use either Least Squares or Theil-Sen estimation. Theil-Sen is more robust against outliers and is particularly useful when signal data is contains noise or measurement artifacts. The regression is performed on the signal measurements assigned to filtered bases that meet specific criteria:

- Dwell time within specified percentile bounds (to exclude extreme or unreliable measurements)

- Signal deviation above a minimum threshold from the mean level (to exclude bases that contribute limited information for rescaling)

- Exclusion of bases from sequence ends (to avoid boundary effects)

### Rough rescaling
Rough rescaling provides an optional initial parameter adjustment before the main refinement iterations. Instead of using all signal measurements, it operates on quantiles of both the measured signal and expected levels, making it computationally efficient while providing a good initial estimate.

The process:
1. **Signal preprocessing**: Clips unwanted bases from sequence ends and optionally uses base centers rather than full signal regions for each base

2. **Quantile calculation**: Computes specified quantiles (default: 5th, 10th, 15th, ..., 95th percentiles) for both normalized signal and expected levels

3. **Regression**: Applies either Least Squares or Theil-Sen estimation to the quantile pairs to estimate new shift and scale parameters 

This approach significantly reduces the computational cost while providing parameters that bring measured and expected signal levels closer together, reducing the number of refinement iterations needed.


### Boundary optimization
The core alignment refinement adjusts the mapping between sequence positions and signal boundaries through a dynamic programming approach:

1. **Band computation**: A constrained search space (band) is computed around the initial alignment to limit the dynamic programming search area. The band is initially calculated in signal coordinates, then converted to sequence coordinates for the algorithm. The default bandwidth is ±5 bases around each assigned position.

2. **Dynamic programming**: A banded dynamic programming algorithm traverses the alignment space within the computed band, scoring signal points based on their correspondence to texpect k-mer levels. If the dwell-penalty algorithm is used calculated scores can be adjusted based on the dwell time, discouraging alignments that contain bases with short dwell times. 

3. **Traceback**: The optimal path is reconstructed from the computed scores


### Iterative refinement strategy
The iterative refinement process repeats the process of:
1. Signal normalization using current normalization parameters
2. Boundary optimization via dynamic programming
3. Parameter recalibration on new boundaries
4. Repeat until set number of iterations elapsed

This interactive approach progressively improves alignment accuracy by allowing the boundary positions and normalization parameters to co-evolve, leading to better overall signal-to-sequence correspondence.