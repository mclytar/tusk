# Steps to reproduce server configuration

### Git CI/CD

Set up the bare repository on the server machine:
```shell
$ ssh user@server.com
$ mkdir tusk.git
$ cd tusk.git
$ git init --bare
$ git branch -m main
$ nano hooks/post-receive
```
Use this as a post-receive hook:
```shell
#!/bin/bash
TARGET="/home/mclytar/deploy/tusk/"
GIT_DIR="/home/mclytar/tusk.git"
BRANCH="main"

while read oldrev newrev ref
do
    if [ "$ref" = "refs/heads/$BRANCH" ];
    then
        echo "Ref $ref received. Deploying ${BRANCH} branch to production..."
        git --work-tree=$TARGET --git-dir=$GIT_DIR checkout -f
        # Insert here the Rust build command and the installation command.
    else
        echo "Ref $ref received. Doing nothing: only the ${BRANCH} branch may be deployed."
    fi  
done
```
Finally, make the file executable and return to the shell and add the remote:
```shell
$ chmod +x hooks/post-receive
$ exit
$ git remote add remote_name ssh://user@server.com/~/tusk.git
$ git push remote_name main
```