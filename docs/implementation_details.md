# Implementation details

This page provides a overview of the general code structure. It lists all modules along with high-level explanations. 

Fishnet is structured in the following modules:

- [cli](#cli)
    - [parse](#cliparse)
    - [execute](#cliexecute)
    - [output](#clioutput)
- [core](#core)
    - [loader](#coreloader)
    - [alignment](#corealignment)
    - [refinement](#corerefinement)
        - [kmer_table](#kmer_table)
        - [settings](#settings)
        - [signal_map_refiner](#signal_map_refiner)
        - [refinement_core](#refinement_core)
            - [bands](#bands)
            - [dp_algorithm](#dp_algorithm)
- [error](#error)
    - [cli_errors](#errorcli_errors)
    - [loader_errors](#errorloader_errors)
    - [alignment_errors](#errorsalignment_errors)
    - [refinement_errors](#errorsrefinement_errors)
    - [output_errors](#errorsoutput_errors)
- [logger](#logger)


## Module descriptions

### cli

The `cli` module handles all front-end functionalities, including the command line parsing (`parse`), the entry point for the alignment (`execute`) and the output writing (`output`).

#### parse

#### execute

#### output


### core

The `core` module contains all logic for loading bam and pod5 files (`loader`), performing an initial alignment (`alignment`) and the refinement of this alignment (`refinement`).

#### loader

#### alignment

#### refinement

##### kmer_table

##### settings

##### signal_map_refiner

##### refinement_core

###### bands

###### dp_algorithm


### logger

The `logger` module initializes the logging features based on the user input.

### error

The `error` module contains the error types implemented in the other modules and allow proper error handling. Custom error types are implemented for the command line interface (`cli_errors`), the bam/pod5 loading (`loader_errors`), the alignment (`alignment_errors`), the refinement (`refinement_errors`) and the output writing (`output_errors`).

If an error occurs it is caught in the main execution function and handled according to its severity.

#### cli_errors

#### loader_errors

#### alignment_errors

#### refinement_errors

#### output_errors



## Tests

