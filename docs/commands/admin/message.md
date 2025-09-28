# Message Command.

Allows you to manage messages.

## `/message edit`
**Parameters**

- `message_id` — The message you want to edit.
- `content?` — The contents of the message.
- `embeds?` — The embeds of the message.

Edits a message sent by the bot. The message cannot be a message that is replying to another message or a command. It can only edit "regular" messages. 

## `/message send`
**Parameters**

- `content?` — The contents of the message.
- `embeds?` — The embeds of the message.

Sends a new message and not show that it came from a command.

## `/message analyze`
**Parameters**

- `message_id` — The message you wanna analyze.

Sends the raw data of the message.