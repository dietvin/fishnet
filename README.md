![fishnet_logo](docs/images/fishnet_logo_wide_cropped.png)

![CI](https://img.shields.io/github/actions/workflow/status/dietvin/fishnet/build.yml?branch=main) ![Release](https://img.shields.io/github/v/release/dietvin/fishnet) ![Downloads](https://img.shields.io/github/downloads/dietvin/fishnet/total) [![Docs](https://img.shields.io/badge/docs-online-blue)](https://dietvin.github.io/fishnet/) ![License](https://img.shields.io/github/license/dietvin/fishnet)

## TL;DR

Signal-to-sequence alignments like [Remora](https://github.com/nanoporetech/remora), but faster and more accessible. [Download fishnet](https://github.com/dietvin/fishnet/releases/latest), extract the binary and run the `align` command:
```bash
./fishnet align --help
```

For further processing, run the `reformat` command:
```bash
./fishnet reformat --help
```

## Documentation

Detailed documentation is provided [here](https://dietvin.github.io/fishnet).

## Repository structure

The code-base is split into different libraries:
- [`fishnet`](fishnet/): Contains the entry point to the command line interface
- [`alignment`](alignment/): Contains the signal-to-sequence alignment logic
- [`reformat`](reformat/): Contains the reformatting logic
- [`pod5_reader_api`](pod5_reader_api/): Contains the logic for accessing pod5 data
- [`helper`](helper/): Contains helper scripts used in the `alignment` and `reformat` libraries

Beyond the code the repo contains [example data](example_data/) to test fishnet, and detailled [documentation](docs/) of all libraries.

## TODOs

- Output optimization: At the moment, the single writer thread seems to be a bottleneck. It might help to have each worker thread write to separate tempfiles and then merge these into one at the end.
- Look into the differences between varying refinement iterations to determine the best default value

## License

This project is licensed under the GPL3.0 License. See the [LICENSE](./LICENSE) file for details.
