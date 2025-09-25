# Genmeme Command
This command is used to generate memes.

Command is available on the user app.

## `/genmeme brick <user? | image>`
**Parameters**

- `user` — The user to throw the brick. (Can't be used if `image` is used)
- `image` — The image to throw the brick. (Can't be used if `user` is used)

Generates a gif of someone throwing a brick. You cannot use the `user` and `image` parameter at the same time. If none are picked, the `user` parameter will default to you.


## `/genmeme petpet <user? | image>`
**Parameters**

- `user` — The user to be petted. (Can't be used if `image` is used)
- `image` — The image to be petted. (Can't be used if `user` is used)

Generates a gif of someone getting petted. You cannot use the `user` and `image` parameter at the same time. If none are picked, the `user` parameter will default to you.


## `/genmeme caption <caption_type> <user | image> [upper_text | bottom_text] <font_size?> <break_height?> <padding?>`
**Parameters**

- `caption_type` — The type of caption you want. (`WHAT`, `White boxes` and `Overlay text` available)
- `user` — The user you want to be captioned. (Can't be used if `image` is used)
- `image` — The image you want to be captioned. (Can't be used if `user` is used)
- `upper_text` — The text at the top. (Optional if `bottom_text` is used)
- `bottom_text` — The text at the bottom. (Optional if `upper_text` is used)
- `font_size` — The size of the font. 

    Defaults:

    - `caption_type` = `WHAT`: width / 7

    - `caption_type` = `White boxes`: width / 10
    
    - `caption_type` = `Overlay text`: height / 10


- `break_height` — Height of the space between new lines. (Default: font_size / 4)

- `padding` — Empty space around the text.

    Defaults:

    - `caption_type` = `WHAT`: width / 9

    - `caption_type` = `White boxes`: width / 20
    
    - `caption_type` = `Overlay text`: height / 30

Adds captions to an image / users pfp. You must pick only one of the `user` and `image` parameters, you cannot pick both or none. You must also select at least one of the `upper_text` and `bottom_text` parameters, you may pick both if you want.

### Caption types
=== "WHAT"
    ![WHAT](/assets/what_preview.png){ align=left width=500 }
=== "White boxes"
    ![WHAT](/assets/boxes_preview.png){ align=left width=400 }
=== "Overlay text"
    ![WHAT](/assets/overlay_preview.png){ align=left width=500 }
