# Log Command.

Logs for debugging.

## `/log get`

Gives you all log messages from the last 12 hours connected to the server / you.

## `/log add`
**Parameters**

- `message` — The message to add to the logs.

Adds a log message to the logs. You can see it when you run `/log get`, and the webhook logger will have sent it if you have it enabled.

## `/log add_webhook`
**Parameters**

- `webhook_url?` — Url to an existing webhook if you don't want the bot to create a new webhook. (Required if running in DMs) 

Makes the bot use a webhook to send logs.

## `/log remove_webhook`

Makes the bot no longer use a webhook to send logs.