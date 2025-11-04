- Read cargo.toml for edition and use that
- Remove (for now, seemingly trustworthy) unwraps and replace with proper error reporting
- Respond to client capabilities
- Respond to unknown notifications/requests/responses with errors?
- Activate on workspace contains cargo.toml as well
- Maybe try to ensure that file loading does not perform a recursive search; instead lob continuations
  on a queue. Maybe even switch to an async workflow but idk about that
- Extract params using `.extract()` instead of manually serde_json'ing it
- Actually trace type def module paths