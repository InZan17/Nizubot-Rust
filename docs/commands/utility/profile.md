# Profile Command
Command for checking/changing your profile. The things you set on your profile will affect the output of some commands. For example, if you set your preferred time format to the 12-hour clock, the time on some commands will be formatted to your preference.

!!! note annotate
    Command is also available on the user app.


## `/profile check`
**Parameters**

- `user?` — The user you wanna check the profile for. (Default: You)

Checks yours/someone elses profile.

## `/profile clear`
**Parameters**

- `confirmation?` — Needs to be set to True to confirm clear. (Default: False)

Clears your profile data.

## `/profile timezone set`
**Parameters**

- `timezone` — The timezone you want to set on your profile. 

Sets your timezone to your profile.

## `/profile timezone remove`

Removes your timezone from your profile.

## `/profile time_format set`
**Parameters**

- `time_format` — Your preferred time format. (`12-hour clock` | `24-hour clock`) 

Sets your preferred time_format to your profile.

## `/profile time_format remove`

Removes your preferred time_format from your profile.