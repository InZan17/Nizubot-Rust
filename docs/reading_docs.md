# Reading the Documentation
Here's some things you should know when reading the docs.

## Command parameters
Here's how the parameters may be presented when reading about a command:

**Parameters**

- `param1` — This is a normal required parameter.
- `param2?` — If a parameter is optional, it will have a `?` at the end of it.
- `param3 | param4` — Parameters separated by a `|` means you can only use one of them. Inside of discord, they will both be optional, but an error will be given if none/several are picked.
- `param5? | param6` — Same as the previous one, but if you pick none, the default value of the parameter with a `?` will be used.
- `[ param7 | param8 ]` — Parameters separated by a `|` but surrounded by `[ ]` means you can pick more than one of the parameters.

Most user app commands also has an optional `ephemeral` field which decides if the message should be hidden from others. This parameter will not be listed.