# Reformat module

```bash
fishnet reformat [...]
```

The **reformat** module combines alignments (which contain only signal indices) with the corresponding signal values. By calculating **base-wise statistics** or **interpolating** signal segments into a uniform shape, downstream analyses of sequence-to-signal alignments become more accessible.
See [Reformatting strategies](../docs/reformat/reformatting_strategies.md) for details.

To keep processing efficient, reformatting is limited to **bases of interest** - those within provided **reference regions** or **motifs**.
See [Filtering options](../docs/reformat/filtering_options.md).

The reformatted data can be exported in *melted* (long), *exploded* (wide), or *nested* formats, each optimized for different analysis workflows.
See [Output formats and shapes](../docs/reformat/output_formats.md).

All processing and output options can be set using the `reformat` command line interface. 
See [Command line arguments](../docs/reformat/command_line_arguments.md). 

For general usage examples and a detailled example with minimal data, see [General examples](../docs/reformat/examples.md#general-examples) and [Detailled (minimal) processing example](../docs/reformat/examples.md#detailled-minimal-processing-example) 