# Alignment module

```bash
fishnet align [...]
```

The **alignment** module performs the signal-to-sequence aligment (*resquiggling*), assigning chunks of the raw signal to individual bases. Signals can be aligned to both the base-called (query) sequence and (if available) the reference sequence.

<img src="/docs/images/resquiggling.jpg" alt="Resquiggling overview" width="750"/>

The alignment library is accessed from the `fishnet align [...]` command. 
See [Command line arguments](../docs/align/command_line_arguments.md).

Depending on downstream processes, the alignments can be exported with both the corresponding sequences and signals. 
See [Output formats](../docs/align/output_formats.md).

For details about the underlying algorithm, see [Algorithm details](../docs/align/algorithm_details.md).
