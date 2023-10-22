# runer

Why would you need compose if you have rust?

# Motivation

Like every other project, this project was born out of personal need. While I 
was working on one of my personal projects I realized how cumbersome my 
workflow with docker-compose is and decided to write my own "composer". I
really don't like the necessity of looking for documentation and how-tos every
time I need to write a bash script with all its limitations.

"Was it `#!/bin` or `#!/usr/bin` ???? Where the hell is my fogging bash?"

"OK `if [[-x check something]] fi`, hmm should I put space after/before brackets???
Uhmm I think I missed _then_, fog"

"ChatGPT How to check if a package is installed in bash files?"

Don't get me started with docker-compose. How many times did you delete the
quotation marks around the port numbers? And add them back and it magically
worked?

# Current State

Right now, runer is just an executable that can parse a given _.runer_ file
(it is just a yaml file) and imply its own principles to achieve certain tasks.
In the current state, it basically mimics _docker-compose_ with additional
features.

In its current state with 400 lines of heavily poluted code, I've managed most 
of the functionality that **I** need from a CLI tool like _docker-compose_.

# Goals

1. Convert it to a proper _cargo_ tool. Not only a CLI tool, but also a desktop
app maybe. I really want to work with _Tauri_.
2. Make it as engine and environment agnostic as possible. Right now it is
heavily depends on _docker_.

# Disclaimer

This is not an attempt to create neither the next Ansible nor the next K8s :).
I just wanted to create a fun, easy-to-use tool that I know every aspect of it
and I can use in **my local** development environment and maybe in **my 
personal CI/CD**.
