# Join Order Command.

See info related to when people joined a server.

## `/join_order get`
| Parameter | Description |
| :-------: | :---------- |
| `user`? \| `index` | The user / index you wanna check. (Default: `user`: You)

See what order people joined at.

## `/join_order graph`
| Parameter     | Description |
| :-----------: | :---------- |
| `graph_data`  | If you wanna graph when members joined or the total amount of members. (`New members` | `Total members`)
| `graph_type`? | If you want a line graph or a bar graph.<br>Defaults:<br>`graph_data` = `New members`: `Line graph`<br>`graph_data` = `Total members`: `Bar graph` |
| `entries`?    | How many entries you want on the graph. (Default: Auto) |

Get a graph of when people joined / how many members a server had.

!!! note annotate
    The graph only includes users that are currently in the server.
    