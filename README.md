# MVP Rust "SWD" "probe"
*Don't use this, it's just for fun*

Openocd master now supports swd together with the buspirate (on master). The
corresponding protocol is pretty simple and easy to implement. As a bonus it
works solely over serial.

[The Protocol](http://dangerousprototypes.com/docs/Raw-wire_(binary))

This is already really slow on the buspirate. It's a bit of an 
hack on the buspirate, so this just barely "works". They are thinking/working
on a [better protocol](https://github.com/BusPirate/Bus_Pirate/issues/29).

This only implements the bare minimum to get the current (as of 2018-01-19)
OpenOCD version running. It may break at random.

## Ideas for the future
- The ch551 is a mcu with an integrated usb peripheral for less than 35ct.
  Unfortunately 8051 based, but a few readymade usb-cdc examples seem to exist
