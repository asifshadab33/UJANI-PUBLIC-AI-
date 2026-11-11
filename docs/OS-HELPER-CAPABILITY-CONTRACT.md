# Local OS helper capability contract (Batch C)

The contract for Public AI's everyday OS-powered helpers is intentionally
narrower than a general command runner:

- Allow only read-only file/folder search, metadata inspection and duplicate discovery.
- Bind each invocation to a non-empty, bounded, user-selected filesystem scope.
- Deny network access and all writes, deletes, moves, process launches and OS-setting changes.
- Require explicit approval for each invocation and scope.
- Bound results to 5,000 records and execution time to 30 seconds.
- Treat paths, filenames and file contents as untrusted input; never interpolate them into shell commands.
- Prefer native OS search/filesystem APIs rather than duplicating broad OS functionality.

The Python validator checks declarations only. It is not a sandbox and cannot prove
native adapter enforcement. Symlink/junction resistance, cancellation, OS permission
dialogs and platform-specific integration tests remain release gates.
