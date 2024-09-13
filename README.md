## Scriptrunner
This tool lets you run bash commands in response to changes in your linux system.

## Config
In the config file you can specify what command to run and when to run it.

###### Basic syntax:
```
logic statements => command
```

###### Example:
```
var_1 == true && var_2 < 1 => ./log_info.sh
```

###### Reactivity
Sometimes you might want to do something any time a variable changes. This is supported with the ```$:``` operator. The config below will run ```log_info.sh``` any time ```var_1``` changes.
```
$:var_1 => ./log_info.sh
```
This can be also combined with logic for other variables.

## Variable injection
You might want to use a variable's value in the command itself. That's where the ```$:``` operator comes in handy again. With it you can inject the value of any variable into the command.

###### Example
```
$:num_monitors_plugged_in && last_action_was_connect => xrandr --output $:last_monitor_changed --auto
```

The above config line will tell xrandr to set up any newly plugged in monitor.

## Usage

Use ```-v``` to view all the available variables you can use in the config and ```-c``` to change the path to the config variable (for now it is set to the ```config``` file in the parent dir, as this tool is very much still in development).

All the commands are run in your home directory.