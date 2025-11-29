# Reading the Documentation
Here's some things you should know when reading the docs.

## Command parameters
Here's how the parameters may be presented when reading about a command:

| Parameter       | Description |
| :-------------: | :---------- |
| `name`          | This is a normal required parameter. |
| `description`?  | If a parameter is optional, it will have a ``? at the end of it. (Default: Empty) |
| `emoji` \| `role`  | Parameters separated by a \| means you can only use one of them. Inside of Discord, they will both be optional, but an error will be given if none/several are picked. So you can either pick an emoji, or a role, but not none or both. |
| `user`? \| `image` | Same as the previous one, but if you pick none, the default value of the parameter with a ? will be used. For example, the ? on user might mean that it will default to selecting you. (Default: `user`: You) |
| <nobr>[ `upper_text` \| `lower_text` ]<nobr> | Parameters separated by a \| but surrounded by [ ... ] means you can pick more than one of the parameters. So you can now have an upper text, and a lower text, but you still can't have none, unless there's a ? on one of them. |
| `food_type`     | This type has several choices. The choices will always be listed at the end of the description. (`Burger` \| `Pizza` \| `Taco`) |

Most user app commands also has an optional `ephemeral` field which decides if the message should be hidden from others. This parameter will not be listed.