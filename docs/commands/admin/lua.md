# Lua Command.

Allows you to create/manage custom guild commands with Lua.

## `/lua command create`
**Parameters**

- `command_name` — The name of the command.
- `description` — The description of the command.
- `params?` — The parameters of the command. (Default: None)
- `lua_file` — The Lua file which has the code.

Creates a guild command which runs the provided Lua file. The Lua file needs to return a function, and this function will get run every time the command gets called. The function will get 2 parameters: ctx and args. To reply, use `ctx:reply("content")`, and to read the arguments, do `args.argument_name`.

## `/lua command update`
**Parameters**

- `command_name` — The name of the command you want to update.
- `description?` — The description of the command. (Default: Previous description)
- `params?` — The parameters of the command. (Default: Previous params)
- `lua_file` — The Lua file which has the updated code.

Updates a guild command.

## `/lua command delete`
**Parameters**

- `command_name` — The name of the command you want to delete.

Deletes a guild command.

## `/lua command download`
**Parameters**

- `command_name` — The name of the command you want to download.

Sends the Lua file of the guild command.

## `/lua command refresh`

Re-sends the guild command info to Discord in case the commands didn't update properly. 

## `/lua instance restart`
**Parameters**

- `force_quit?` — If you wanna force quit all currently running commands. (Default: False)

Restarts the Lua instance.