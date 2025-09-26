# Detect Message Command.

Allows you to add message detectors with custom responses.

## `/detect_message add`
**Parameters**

- `detect_type` — How the detection should happen. (`Starts with` | `Contains` | `Ends with` | `Equals`)
- `key` — The text that should be detected.
- `response` — The response after detecting it.
- `case_sensitive?` — If the detection should be case sensitive or not. (Default: False)

Creates a new detector that will detect certain messages and respond to them.


## `/detect_message remove`
**Parameters**

- `index` — The index of the detector you wanna remove.

Removes a detector. You can get the index of the detector by running `/detect_message list`. Keep in mind the index of a detector may change if you remove a detector in the middle of the list. May change in the future. Spam me on discord if I haven't.


## `/detect_message list`
Lists all of your current message detectors in the server, or in the bots DMs.