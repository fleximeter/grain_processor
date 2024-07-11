# Grain Processor
This crate performs grain extraction and analysis on a corpus of audio files. It stores the grain metadata, including analysis data, in a SQLite database. This data can be used for algorithmic granular synthesis. It is written in Rust for better performance, since one might conceivably wish to granulate gigabytes of audio files.

There is a configuration file called `config.json` that allows you to specify parameters for the program, such as where the audio files are located.