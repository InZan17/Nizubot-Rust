# Remind Command
This command is used to create/manage reminders.


## `/remind add`
| Parameter  | Description |
| :--------: | :---------- |
| `duration` | When the reminder should go off. (Example: 1s 2m 3h 4d 5w 6y)
| `message`? | Message to send with the reminder.
| `looped`?  | If the reminder should restart once finished. (Default: False)

Create a new reminder.


## `/remind remove`
| Parameter | Description |
| :-------: | :---------- |
| `index`   | The index of the reminder you wanna remove.

Removes a reminder. You can get the index of the reminder by running `/remind list`. Keep in mind the index of a reminder may change, since they are sorted based on when the reminder goes off. May change in the future. Spam me on discord if I haven't.


## `/remind list`
Lists all of your current reminders in the server, or in the bots DMs.
