# Container capabilities


Capabilities needed by the container binary:

  - If the kernel doesn't allow user namespace creation for non-root users,
    CAP_SYS_ADMIN is needed for that.  After the container has been configured,
    it drops all privileges and uses a non-root user id.

  - If CAP_SETUID has been granted, the effective user id is changed to root
    for the duration of cgroup initialization, and a system cgroup is created.
    Otherwise the cgroup is created under the user session.  Note: if other
    capabilities have been set for the binary, the sd-bus library's use of
    secure_getenv(3) prevents it from finding the user bus.


Privileged things controlled by users who can execute a capable container
binary:

  - Choose any cgroup as the parent for the container's cgroup, thus
    circumventing resource controls.

  - Supply the file descriptor used for interacting with the executor process
    inside the container.  It can be used to spawn and kill processes inside
    the container, and execute arbitrary code in the processes.


Environmental factors:

  - The container binary should be executable only by a single, trusted user if
    capabilities have been granted.

  - Configuration of the user namespace is delegated to /usr/bin/newuidmap and
    /usr/bin/newgidmap.

  - The binaries executed inside the container are determined by the location
    of the container binary itself: it looks for the "executor" and "loader"
    files in the same directory where it is located.  The write permissions of
    the directory and the binaries should be limited.  (Note that executor and
    loader don't need capabilities, and they need to have more relaxed read and
    execution permissions.)

  - Cgroup configuration needs to be done via systemd.  A container instance
    gets its own cgroup automatically, but that's it.
