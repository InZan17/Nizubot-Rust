# Genmeme Command
This command is used to generate memes.

!!! note annotate
    Command is also available on the user app.

## `/genmeme brick`
| Parameter          | Description |
| :----------------: | :---------- |
| `user`? \| `image` | The user/image to throw the brick. (Default: `user`: You)

Generates a gif of someone throwing a brick.

### Brick example
![brick](/assets/brick_preview.gif){ align=center }

<br>

## `/genmeme petpet`
| Parameter          | Description |
| :----------------: | :---------- |
| `user`? \| `image` | The user/image to be petted. (Default: `user`: You)

Generates a gif of someone getting petted.

### Petpet example
![petpet](/assets/petpet_preview.gif){ align=center }

<br>

## `/genmeme caption`
| Parameter         | Description |
| :---------------: | :---------- |
| `caption_type`    | The type of caption you want. (`WHAT` \| `White boxes` \| `Overlay text`) |
| `user` \| `image` | The user/image you want to be captioned. |
| [ `upper_text` \| `bottom_text` ] | The text written at the top and bottom. |
| `font_size`?      | The size of the font.<br>Defaults:<br>`caption_type` = `WHAT`: "width / 7"<br>`caption_type` = `White boxes`: "width / 10"<br>`caption_type` = `Overlay text`: "height / 10" |
| `break_height`?   | Height of the space between new lines. (Default: "font_size / 4") |
| `padding`? | Empty space around the text.<br>Defaults:<br>`caption_type` = `WHAT`: "width / 9"<br>`caption_type` = `White boxes`: "width / 20"<br>`caption_type` = `Overlay text`: "height / 30" |

Adds captions to an image / users pfp.

### Caption examples
=== "WHAT"
    ![WHAT](/assets/what_preview.png){ align=left width=500 }
=== "White boxes"
    ![WHAT](/assets/boxes_preview.png){ align=left width=400 }
=== "Overlay text"
    ![WHAT](/assets/overlay_preview.png){ align=left width=500 }
