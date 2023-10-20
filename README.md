# runer

Why would you need compose if you have rust?

# Motivation

Like every other project, this project was born out of personal need. While I 
was working on one of my personal projects I realized how cumbersome my 
workflow with docker-compose is and decided to write my own "composer". I
really don't like the necessity of looking for documentation and how-tos every
time I need to write a bash script with all its limitations.

"Was it `#!/bin or #!/usr/bin? Where the hell is my fogging bash?"

"OK `if [[-x check something]] fi`, hmm should I put space after/before brackets???
Uhmm I think I missed `then`, fog"

"ChatGPT How to check if a package is installed in bash files?"

Don't get me started with docker-compose. How many times did you delete the
quotation marks around the port numbers? And add them back?

# Current State

Right now, runer is just an executable that can parse a given _.runer_ file
(it is just a yaml file) and imply its own principles to achieve certain tasks.
In the current state, it basically mimics _docker-compose_ with additional
features.

# Disclaimer

This is not an attempt to create neither the next Ansible nor the next K8s :).
I just wanted to create a fun, easy-to-use tool that I know every aspect of it
and I can use in my **local** development environment.
