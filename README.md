# Grain Processor
This crate performs grain extraction and analysis on a corpus of audio files. It stores the grain metadata, including analysis data, in a SQLite database. This data can be used for algorithmic granular synthesis. It is written in Rust for better performance, since one might conceivably wish to granulate gigabytes of audio files.

The idea is that we work through each file by extracting a grain, analyzing it, and moving forward *n* samples to extract the next grain. The grains are analyzed for various features, including spectral entropy, spectral flatness, spectral slope, etc.

There is a configuration file called `config.json` in the root of this repository that allows you to specify parameters for the program, such as where the audio files are located and how large the grains should be. If you want your database to store grains of various sizes, you will need to rerun the program for each different grain size you want. The program requires the configuration file. It should be located in the same directory as the program executable.
