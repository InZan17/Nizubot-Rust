# Manage Reminders Command.

Used to moderate user reminders. If a user creates an inappropriate reminder, you'll be able to remove it with this command.

## `/manage_reminders peek`
| Parameter | Description |
| :-------: | :---------- |
| `user`    | The user you wanna check the reminders for. |

Checks which reminders a user has. The command will be ephemeral, so no one will see you ran it.

## `/manage_reminders remove`
| Parameter | Description |
| :-------: | :---------- |
| `user`    | The user you wanna remove a reminder from. |
| `index`   | The index of the reminder you wanna remove. |

Removes a reminder from a user. The command will be ephemeral, so no one will see you ran it.