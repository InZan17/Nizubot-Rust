# Sleep Calc Command.

Calculates the best time to go to sleep / wake up.
The average human takes around 15 minutes to fall asleep. Once you are asleep you will go through sleep cycles. One sleep cycle is about 90 minutes and a good night's sleep consists of 5-6 sleep cycles. It's best to wake up at the end of a cycle to help you feel more rested and ready to start the day.\nI will calculate the best time for you to sleep/wake up by using this information.

## `/sleep_calc sleep`
**Parameters**

- `wake_hour` — The hour of the time you wanna wake up.
- `wake_minute` — The minute of the time you wanna wake up.
- `time_format` — The format the time is in. (`AM` | `PM` | `24h clock`)
- `sleep_duration?` — The amount of minutes it takes for you to fall asleep. (Default: 15)
- `cycle_length?` — How many minutes a sleep cycle takes for you. (Default: 90)

Calculates the best time to go to sleep.


## `/sleep_calc wake`
**Parameters**

- `sleep_hour` — The hour of the time you wanna go to sleep.
- `sleep_minute` — The minute of the time you wanna go to sleep.
- `time_format` — The format the time is in. (`AM` | `PM` | `24h clock`)
- `sleep_duration?` — The amount of minutes it takes for you to fall asleep. (Default: 15)
- `cycle_length?` — How many minutes a sleep cycle takes for you. (Default: 90)

Calculates the best time to go to wake up.


## `/sleep_calc info`

Gives info about how the results are calculated. Basically the same at the text at the top of this page.
