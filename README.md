# VR Hass Power

Control a Home Assistant entity based on if SteamVR is running.

Upon running for the first time, it will prompt you for your Home Assistant URL
and API key. It will then monitor your system for if a specific process is
running (by default 'vrserver.exe' but can be changed in config). When this
process is launched it will turn on the specified entity and when the process
exits it will wait some number of seconds to ensure it is not relaunched (by
default 60 seconds) before turning the entity off.

You can run it as a Windows service with something like [NSSM](https://nssm.cc).
