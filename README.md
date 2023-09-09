# Tusk home server

This is another attempt to build a web server for my home. The server will contain:
- A simple cloud for file storage
- An admin dashboard for system statistics
- Some simple but useful apps.
- ... _And many more_ (?)

Probably, this will be replaced in the future by a fully functioning instance of `mammoth` web server (what is that?
Well, I still need to define some details about that).

## Tasks before publication (on `crates.io`, GitHub and the production server).

- [x] Add roles.
  - [x] Role `admin`: can see the Dashboard.
  - [x] Role `directory`: can have a Directory.
  - [x] Role `user`: can see the apps and the Support tab.
- [x] Add check on startup for whether all `directory` roles have a Directory.
- [x] Add watch and self-reload for Unix.
- [x] Add icon type on configuration file.
- [x] Add `/settings` page, and allow users to change their password.
- [x] Check that everything is consistent and safe on the backend part.
  - [x] Unit tests on `tusk-core`.
  - [x] Benchmark tests on `tusk-core` (to check that fake password hashing is consistent with real password hashing).
  - [x] ~~Unit tests on `tusk-admin`~~ (to be done, but not fundamental for now).
  - [x] Unit tests on `tusk-server`.
- [x] **Done with the backend!**
- [ ] Adjust icons handling.
- [ ] Make sure that premium icons don't leak.
- [ ] Check that everything is consistent and safe on the frontend part.
- [ ] **Done with the frontend!**
- [ ] `git push` on development machine.
- [ ] Check that everything works on development and fix if not.
- [ ] `git tag -a 0.1`
- [ ] `git push` on GitHub and on production machine.
- [ ] `git checkout -b develop`
- [ ] Start developing new features! (Each one in its own branch).
  - *The most urgent features are:*
  - [ ] Tech support.

## Roadmap

This software will run on an Ubuntu Server machine and will therefore be tailored to it. Furthermore, this software
will be tested on a Windows machine, hence some functionalities are available for that OS too. No other operating
system is officially supported; however, I suspect that many functionalities will work in any Linux distro as well.

Said that, this is a first roadmap for the server construction.
- [x] Build a fully functioning server serving a trivial HTTP request.
  - [x] Implement log functionalities.
- [x] ~~Make the server work on a virtual Ubuntu Server machine in a fully autonomous fashion.~~
  - [x] This cannot be done because a root account is needed to bind to port 80. 
  - [x] Take note of the steps so that it is possible to reproduce or automate the procedure.
- [x] ~~Implement CI/CD.~~ Partially done: the script works, but no automatic installation can happen at this point. 
- [x] **Checkpoint #1:** everything works as intended!
  - At this point, `tusk-admin` is still empty, `tusk-backend` contains the necessary data structures to make things
    work and `tusk-server` contains the necessary boilerplate to make the server run.
  - **NOTE:** HTTPS is **NOT** needed at this point! HTTPS will be mandatory in production (with the only option for
    HTTP being as redirect to HTTPS or ACME functionalities).
  - **NOTE:** additionally, custom options are not needed at this point. They will be implemented in a later point,
    so to make service/daemon integration with the system better.
- [x] Implement a simple REST functionality.
- [x] Implement unit/integration tests.
- [x] Integrate `tera` into the project so that it is possible to build dynamic pages.
- [x] Implement `/` using Bootstrap.
  - `/` will contain a dummy page with a side menu and a top menu.
  - The side menu will contain the following items:
    - User info (in a box) 
    - Home
    - Dashboard
    - Cloud
    - Tasks
    - Shopping list
    - Log Out (at the bottom of the menu)
  - Everything will be pretty static, no need to implement anything at this stage.
- [x] Implement `/login` using Bootstrap.
  - `/login` will be a special page which only contains the login form.
  - The `/login` functionality will not work at this stage.
- [x] Implement `/v1` as the API entrypoint.
  - [x] Implement the `/v1/session` resource as follows:
    - [x] Upon `POST`, respond positively upon input `username=dummy&password=dummy` and negatively
      otherwise. In the future, this will authenticate the user.
    - [x] Upon `GET`, retrieve the information `{ username = "dummy" }`.
    - [x] Upon `DELETE`, delete the session cookie.
  - **NOTE:** do **NOT** implement anything more complex than this yet. Furthermore, do not check whether the session is
    expired or not. It is not needed at this stage.
- [x] Implement authentication via `/v1/session` and the `/login` page.
- [x] Implement session checks and redirect to `/login` if there is no valid session.
- [x] **Checkpoint #2:** everything works as intended!
  - At this point, `tusk-admin` is still empty, `tusk-backend` contains a bit more stuff (mainly to handle REST
    resources and session cookies management) and `tusk-server` contains the necessary code to make the server work.
  - Now we need to start implementing the main things and, for this, we need to refresh the `tera` templates a lot.
- [x] Implement service/daemon functionalities for Windows.
- [x] Implement the following commands:
  - [x] `tusk install` -- Installs the server service/daemon.
  - [x] `tusk start` -- Starts the server service/daemon.
  - [x] `tusk stop` -- Stops the server service/daemon.
  - [x] `tusk uninstall` -- Uninstalls the server service/daemon.
  - [x] `tusk reload` -- Reloads the `tera` template pages.
- [x] Move HTML/`tera` files to `/srv/http/`.
  - This will be automatically done by some script in the post-receive hook.
  - On windows[development], a strategy would be to have two separate scripts `confirm` and `revert` to respectively
    confirm and revert the changes.
- [x] After checking that everything works, implement service/daemon functionalities for Ubuntu.
- [x] Update CI/CD to uninstall the old version of the server and install the new version.
- [x] Add configuration files.
- [x] Run Redis and connect to it to store session cookies.
- [x] **Checkpoint #3:** everything works as intended!
  - At this point, `tusk-admin` finally contains the code to manage the `tusk-server` service. Also, now `tusk-server`
    is a service.
- [x] ~~Create library `tusk-database`~~ _Use the library `tusk-backend`, since it is unused so far_.
- [x] Implement privilege drop for security reasons.
- [x] Install the database `tusk` on both Windows and Ubuntu Server.
  - [x] Implement `diesel`.
  - [x] Database `tusk` will initially contain a table `user` with columns `user_id`, `username`, `password`.
- [x] Implement the following commands:
  - [x] `tusk user add <username>` -- Creates a new user (asking for a password).
  - [x] `tusk user list` -- Lists all users.
  - [x] `tusk user delete <username>` -- Delete the user `<username>`.
- [x] Properly implement `/v1/session`.
- [x] ~~Write a script to create a dummy certificate; the dummy certificate will not be authenticated, but this is not
  important at this stage. The certificate creation and usage will match the specification of the `acme-client` crate.~~
  Write the necessary steps in the `REPRODUCE.md` file for creating a certificate.
- [x] Integrate HTTPS.
  - Ignore the insecure warning.
- [x] **Checkpoint #4:** everything works as intended!
  - At this point, `tusk-admin` contains the functionalities to manage `tusk` and to handle users, `tusk-database`
    contains the starting point for developing the server data structures and code, and `tusk-backend` and `tusk-server`
    will be still focused on backend and server functionalities as usual.
  - The last functionality that is OS dependant is the one for cloud space. The last checkpoint will be dedicated to
    this.
- [x] Implement `/directory` using Bootstrap.
- [x] Implement directory browsing functionalities.
  - [ ] For every user, create the respective cloud root directory in `/srv/directory/`.
    - Example: the user `dummy` will have its cloud contents stored in `/srv/directory/dummy/*/**`.
  - [x] There will be a special directory `/srv/directory/.public/` accessible to every user.
- [ ] Final tests -- check that all the unit and integration tests are successful and that everything works as intended.
- [ ] `git tag -a 0.1`
- [ ] `git checkout -b develop`
- [ ] **Checkpoint #5:** everything works as intended! Bonus: you have a private cloud!
  - From now on, new functionalities are added in branch `feature-<functionality_name>`. When preparing new releases,
    they will be put in branch `release-<version_number>`, with the first commit being a version bump.
  - **NOTE:** all commits to branch `master` will trigger the CI/CD functionality (i.e. a simple git hook).
  - This can also be installed into a live server.
- [ ] **TODO:** continue the roadmap (I guess there is plenty of time to understand what is going to happen next).

## License

This software will probably be licensed under MIT or dual MIT/Apache licensing; however, considering the current stage
of the project, there is still plenty of time to decide.