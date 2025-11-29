# Guild commands

One thing you can do with the [`/lua`](/commands/admin/lua/) command is to create and manage your own guild commands using Lua.

## Creating commands

To create a guild command, you can use [`/lua command create`](/commands/admin/lua/#lua-command-create). There's 2 required parameters. Those are for the name of the command, and for the file containing the code for the command. The file can be either a `.lua` or  `.luau` file. There's also 2 optional parameters. One is for the description of the command, and the other is the parameters for the command.

### Writing the code for the command

Below is an example of a command. This command will get the celsius parameter from the user and return how many fahrenheit it's equal to.
```lua
local function celsiusToFahrenheit(celsius)
    return celsius * (9/5) + 32
end

return function(ctx, args)
    local fahrenheit = celsiusToFahrenheit(args.celsius)
    ctx:reply(args.celsius.." celsius is equal to "..fahrenheit.." fahrenheit.")
end
```
This Lua code returns a function, and this function will be run every time the command gets called. The first parameter is a CommandContext which is used to reply to the command. The second parameter is just a table with all the parameters the user used when running the command.

### Writing the parameters for the command

The `params` field accepts a string which describes all the parameters for the command. Here's an example string for the command above:
```
name=celsius:type=number:description=The amount of celsius you want to convert:required=true
```
To set a property, you put the name of the property, followed by a "=" and then the value of the property. Each property is separated by a ":". If you want more than one parameter, you separate them using ";". Here's an example of having several parameters:
```
name=param1:type=number; name=param2:type=integer; name=param3:type=string;
```
The last ";" and the spaces are optional. The spaces are there to improve readability.

#### Parameter properties
Here are the valid properties for a parameter:

| Property     | Description |
| :----------: | :---------- |
| name         | The name of the parameter. |
| type         | The type of the parameter. See [Parameter types](#parameter-types) for more info. |
| description? | The description of the parameter. |
| required?    | If the parameter should be required or not. |

#### Parameter types
Here are the valid types for a parameter:

| Type            | Description |
| :-------------: | :---------- |
| bool \| boolean | A true/false value. |
| integer         | A whole number. |
| number          | Any number. |
| string          | A text value. |

## Updating commands

To update a guild command, you can use [`/lua command update`](/commands/admin/lua/#lua-command-update). There's only one required parameter, which is the name of the command you want to update. Every other parameter is optional. If you don't use a parameter, the current value will be used instead.