# Genmeme Command
This command is used to generate memes.

Command is available on the user app.

## `/genmeme brick`
**Parameters**

- `user? | image` — The user/image to throw the brick.

Generates a gif of someone throwing a brick.

### Brick example
![brick](/assets/brick_preview.gif){ align=center }

<br>

## `/genmeme petpet`
**Parameters**

- `user? | image` — The user/image to be petted.

Generates a gif of someone getting petted.

### Petpet example
![petpet](/assets/petpet_preview.gif){ align=center }

<br>

## `/genmeme caption`
**Parameters**

- `caption_type` — The type of caption you want. (`WHAT` | `White boxes` | `Overlay text`)
- `user | image` — The user/image you want to be captioned.
- `[ upper_text | bottom_text ]` — The text written at the top and bottom.
- `font_size?` — The size of the font. 

    Defaults:

    - `caption_type` = `WHAT`: width / 7

    - `caption_type` = `White boxes`: width / 10
    
    - `caption_type` = `Overlay text`: height / 10


- `break_height?` — Height of the space between new lines. (Default: font_size / 4)

- `padding?` — Empty space around the text.

    Defaults:

    - `caption_type` = `WHAT`: width / 9

    - `caption_type` = `White boxes`: width / 20
    
    - `caption_type` = `Overlay text`: height / 30

Adds captions to an image / users pfp.

### Caption examples
=== "WHAT"
    ![WHAT](/assets/what_preview.png){ align=left width=500 }
=== "White boxes"
    ![WHAT](/assets/boxes_preview.png){ align=left width=400 }
=== "Overlay text"
    ![WHAT](/assets/overlay_preview.png){ align=left width=500 }
