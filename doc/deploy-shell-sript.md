# Deploy shell-script

Create a deploy.sh script where to write all your commands for deploying. The newest version of the .war file will always be 
named ARTIFACTORY.war.

### Important

- Give the .sh script execute rights (`chmod -x deploy.sh` or `chmod 777 deploy.sh`)
- Make sure the shell script starts with a Shebang-line (`#!/bin/bash`) to avoid rust-execute-errors
- In the script you have access to latest version via env-variable:  `$ARTIFACTORY_VERSION`