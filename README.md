ArmA3-EpochServer
=================

ArmA3 Epochmod Server Library

How to build on Linux
--------------------------------

need to install packages: 
libhiredis-dev
libpcre3-dev


How to build DLL (Visual Studio)
--------------------------------

1. `git submodule update --init`
2. Download latest PCRE version from [Airesoft](http://www.airesoft.co.uk/pcre) and extract the content in `/Assets/EpochServer/deps/pcre-win`
2. Open `RedisServer.sln` in `/Assets/EpochServer/deps/redis/msvs`
3. Compile all Projects with the same config (x32|x64 Debug|Release) as EpochServer
4. After the RedisServer dependency is successfully compiled open `EpochServer.sln` in `/Assets/EpochServer/msvs`
5. Compile

Call summary
------------

Syntax: [Group][Operator][Flag]

* 0 (Init)
	* 0 
		* 0  (Get and return Instance ID from config)
		* 1  (STEAMAPI - Vac ban check) 
* 1 (Setter)
	* 0 (Temporarily stack, workaround for calls which are too long)
		* 0 (Syncron)
		* 1 (Asyncron)
	* 1 (Redis call SET)
		* 0 (Syncron)
		* 1 (Asyncron)
	* 2 (Redis call SETEX)
		* 0 (Syncron)
		* 1 (Asyncron)
	* 3 (Redis call EXPIRE)
		* 0 (Syncron)
		* 1 (Asyncron)
	* 4 (Redis call SETBIT)
		* 1 (Asyncron)
* 2 (Getter)
	* 0 (Redis call GET)
		* 0 (Syncron)
	* 1 (Redis call GET + TTL)
		* 0 (Syncron)
	* 2 (Redis call GETRANGE)
		* 0 (Syncron)		
	* 4 (Redis call GETBIT)
		* 0 (Syncron)
* 3 (TTL)
	* 0 (Redis call TTL)
		* 0 (Syncron)
* 4 (Delete)
	* 0 (Redis call DEL)
		* 0 (Syncron)
* 5 (Utilities)
	* 0 (Redis call PING)
		* 0 (Syncron)
	* 1 (Get current time, [YYYY,MM,DD,HH,MM,SS])
		* 0 (Syncron)
* 6 (Array)
	* 0 (Redis call LPOP, with `CMD` prefix)
		* 0 (Syncron)
* 7 (Logging)
	* 0 (Log in Redis)
		* 0 (Syncron)
		* 1 (Asyncron)
* 8 (Antihack)
	* 0 (Update publicvariable.txt)
		* 0 (Syncron)
		* 1 (Asyncron)
	* 1 (Get random string, [a-zA-Z]{23-30}, if only one string is requested it will return a string instead of a array)
		* 0 (Syncron)
	* 2 (Add ban to bans.txt)
		* 0 (Syncron)
		* 1 (Asyncron)
* 9 (Battleye)
	* 0 (Broadcast message to server)
		* 1 (Asyncron)
	* 1 (Kick with message)
		* 1 (Asyncron)
	* 2 (Ban with message and duration)
		* 1 (Asyncron)
	* 3 (Lock/Unlock server)
		* 0 (Unlock)
		* 1 (Lock)
	* 9 (Shutdown Server)
		* 1 (Asyncron)

EpochServer.ini Guide
---------------------

* `[EpochServer]`
	* `BattlEyePath` This is the path to the battleye folder which is needed for the AntiHack [default: "PROFILEPATH/battleye"]
	* `InstanceID` Current server instance for the database [default: "NA123"]
	* `LogAbuse` Enables abuse logging [default: 0, 0: none, 1: redis key, 2: redis key & value]
* `[Redis]`
	* `IP` Redis server ip/hostname [default: 127.0.0.1]
	* `Port` Redis server port 0-65535 [default: 6379]
	* `DB` Database index [default: 0]
	* `Password` Password [default: `<no password>`]
* `[SteamAPI]`
	* `Logging` Enable logging for SteamAPI [default: 0, 0: disabled, 1: ban reason, 2: info/debug]
	* `Key` Steam Web API key (can be requested here: http://steamcommunity.com/dev/apikey), if no key is given the SteamAPI is disabled [default: `<no key>`]
	* `VACBanned` Players with a vac ban will be banned by writing the ban in bans.txt [default: 0, 0: disabled, 1: enabled]
	* `VACMinimumNumberOfBans` Players with the given minimun of vac bans will be banned, same as VACBanned [default: 0, 0:  disabled]
	* `VACMaximumDaysSinceLastBan` Players will be banned until the VAC ban will reach the maximum, same as VACBanned [default: 0, 0: disabled]
	* `PlayerAllowOlderThan` Player will be banned if the account creation date is younger than the allowed value (in seconds) [default: 0, 0: disabled]
