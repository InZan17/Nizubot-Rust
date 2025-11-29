# Reaction Role Command.

Allows you to manage reaction roles. If you can, try using Discords onboarding feature instead of this.

## `/reaction_role add`
| Parameter    | Description |
| :----------: | :---------- |
| `message_id` | The message you want to to add the reaction role to. |
| `emoji`      | The emoji to react with. |
| `role`       | The role to give. |

Creates a reaction role for the specified message.

## `/reaction_role remove`
| Parameter    | Description |
| :-------:    | :---------- |
| `message_id` | The message you wanna remove the reaction role from. |
| <nobr>`emoji` \| `role`<nobr> | The reaction role you wanna remove. Will remove whichever one has the emoji / role connected to it. |

Removes a reaction role from a message.

## `/reaction_role list`
| Parameter     | Description |
| :-------:     | :---------- |
| `message_id`? | The message you wanna check reaction roles for. |

Lists all reaction roles in the server / message.