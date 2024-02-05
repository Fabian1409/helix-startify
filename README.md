# helix-startify
Helix wrapper that mimics the neovim startify plugin.
Given a path as arg, add it to the recents list and then exec hx.
Without args, opens startify.

## setup
Add the helix-startify binary to `$PATH` by placing it in `~/.cargo/bin`

### fish
```fish
alias hx "helix-startify"
```

