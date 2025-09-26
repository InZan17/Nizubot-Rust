# COTD Role Command.

Allows you to have roles that changes based on the current COTD. (color of the day)

## `/cotd_role create`
**Parameters**

- `name?` — The name of the role. (Default: "&lt;cotd&gt;")
- `role?` — The role that will be updated.

Creates a new COTD role. If you use the `name` parameter, if you add "&lt;cotd&gt;" it will be replaced by the name of the current daily color. If the `role` parameter is left empty, Nizubot will create a new role.

## `/cotd_role remove`
**Parameters**

- `delete?` — If the role should be deleted from the guild. (Default: False)

Stops updating the current COTD role. If you set `delete` to True, it will also delete the role from the guild.