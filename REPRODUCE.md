# Steps to reproduce server configuration

## Git CI/CD

Set up the bare repository on the server machine and push the current one to the remote one:
```shell
$ ssh user@server.com
$ mkdir tusk.git
$ cd tusk.git
$ git init --bare
$ git branch -m main
$ exit
$ git remote add remote_name ssh://user@server.com/~/tusk.git
$ git push remote_name main
```
Then, log again into the remote machine, unpack the repository and complete the setup:
```shell
$ ssh user@server.com
$ mkdir deploy
$ mkdir deploy/tusk
$ mkdir install
$ mkdir install/tusk
$ sudo mkdir /etc/tusk
$ git --work-tree=./deploy/tusk/ --git-dir=./tusk.git checkout -f
$ cp deploy/tusk/post-receive tusk.git/hooks/
$ cp deploy/tusk/install install/tusk/
$ cp deploy/tusk/tusk.toml /etc/tusk/
$ chmod +x tusk.git/hooks/post-receive
$ chmod +x install/tusk/install 
```
Finally, enable the `install` script to be run as root without asking password.
Namely, add the following line using `visudo`:
```
user ALL=(ALL) NOPASSWD: /home/user/install/tusk/install
```
**Tip:** to run `visudo` with nano, use `export EDITOR=nano;`

## Compilation

The following software is necessary to correctly compile and run Tusk.

### Packages
```shell
$ sudo apt install pkgconf libsystemd-dev libssl-dev postgresql libsql-dev
```

## Settings

Settings can be found and edited at `/etc/tusk/tusk.toml`.

## Database configuration

First of all, we need to grant the main user access to postgres in an easy way:
```shell
$ sudo -u postgres createuser -s $USER
$ createdb
$ psql
```
