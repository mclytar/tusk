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

### Installation

The following packages are needed.
```shell
$ sudo apt install pkgconf libsystemd-dev libssl-dev postgresql libsql-dev
```
The server process, and all of its resources, should belong to a special user `tusk`, which can be created with
```shell
$ sudo useradd -r -s /sbin/nologin tusk
```

## Settings

Settings can be found and edited at `/etc/tusk/tusk.toml`.

## Database configuration

First of all, we need to grant the main user access to postgres in an easy way:
```shell
$ sudo -u postgres createuser -s $USER
$ createdb
```
Similarly, we need to create the `tusk` user:
```shell
$ sudo -u postgres createuser -s tusk -P
$ createdb -O tusk tusk
```

## Certificate generation

For now, the server is local, hence we cannot use Let's Encrypt or similar.
However, we can impersonate a Certificate Authority:
```shell
$ openssl genrsa -des3 -out myCA.key 4096
$ openssl req -x509 -new -nodes -key myCA.key -sha256 -days 365 -out myCA.pem
```
This creates the files `myCA.key`, i.e., the private key, and `myCA.pem`, i.e., the certificate file, for the
Certificate Authority.
Now, let's create and sign a certificate.
Write a file `tusk.ext` with the following content.
```ini
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
```
Then, run the following commands:
```shell
$ openssl genrsa -out tusk.pem 4096
$ openssl req -new -key tusk.pem -out tusk.csr
$ openssl x509 -req -in tusk.csr -CA myCA.pem -CAkey myCA.key -CAcreateserial -out tusk.crt -days 365 -sha256 -extfile tusk.ext
$ openssl pkcs8 -topk8 -inform PEM -outform PEM -nocrypt -in tusk.pem -out tusk.key
```
Now the private key can be found in `tusk.key` and the certificate can be found in `tusk.crt`.