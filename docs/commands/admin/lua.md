# Lua Command.

Allows you to create/manage custom guild commands with Lua.

## `/lua command create`
| Parameter      | Description |
| :------------: | :---------- |
| `command_name` | The name of the command. |
| `description`  | The description of the command. |
| `params`?      | The parameters of the command. (Default: None) |
| `lua_file`     | The Lua file which has the code. |

Creates a guild command which runs the provided Lua file. The Lua file needs to return a function, and this function will get run every time the command gets called. The function will get 2 parameters: ctx and args. To reply, use `ctx:reply("content")`, and to read the arguments, do `args.argument_name`.

## `/lua command update`
| Parameter      | Description |
| :------------: | :---------- |
| `command_name` | The name of the command you want to update. |
| `description`? | The updated description of the command. (Default: Previous description) |
| `params`?      | The updated parameters of the command. (Default: Previous params) |
| `lua_file`     | The updated Lua file which has the code. |

Updates a guild command.

## `/lua command delete`
| Parameter      | Description |
| :------------: | :---------- |
| `command_name` | The name of the command you want to delete. |

Deletes a guild command.

## `/lua command download`
| Parameter      | Description |
| :------------: | :---------- |
| `command_name` | The name of the command you want to download. |

Sends the Lua file of the guild command.

## `/lua command refresh`

Re-sends the guild command info to Discord in case the commands didn't update properly. 

## `/lua instance restart`
| Parameter     | Description |
| :-----------: | :---------- |
| `force_quit`? | If you want to force quit all currently running commands. (Default: False) |

Restarts the Lua instance.