**Epoch Server Framework**
=================

Custom Redis http://redis.io/ extension for ArmA 3
Windows and Linux compatible (DLL/so)

By Epoch Mod Team
http://epochmod.com/

--------------------------
Developers
--------------------------
* Aaron Clark - [VB]AWOL - Code
* Florian Kinder - Fank - Code
* Denis Erygin - devd - Code

---------------------------
Install Notes
---------------------------
Hive DLL requires vs2013 redist: http://www.microsoft.com/en-us/download/details.aspx?id=40784

---------------------------
Resources
---------------------------
* Redis - https://github.com/antirez/redis
* MSOpenTech (Redis for Windows) - https://github.com/MSOpenTech/Redis
* HappyHttp - http://scumways.com/happyhttp/happyhttp.html
* rapidjson - https://github.com/miloyip/rapidjson/
* pcre-win - http://www.airesoft.co.uk/pcre


How to build on Linux
--------------------------------

apt-get install build-essential git libhiredis-dev libpcre3-dev

git clone https://github.com/EpochModTeam/EpochServer.git\

git submodule update --init

cd EpochServer/

make install

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


Calls Documentation
---------------------

Full A3 epoch server hive call documentation

**000**

Get and return Instance ID from config

_Input: None_

_Returns: [ARRAY](https://community.bistudio.com/wiki/Array) format: ["NA123"]_


```
"epochserver" callExtension "000";
```


**001**

STEAMAPI - Vac ban check

_Input: [SteamID64](https://community.bistudio.com/wiki/getPlayerUID)_

_Returns: Nothing_


```
"epochserver" callExtension format["001|%1", _uid];
```

**110** 

Store value in database with the specified key


_Uses:  SET_

_Call Type: Synchronous_



_Input: [Key](http://redis.io/topics/data-types-intro), Call Index ID, Value_

_Returns: Nothing_


```
"epochserver" callExtension format["110|%1:%2|%3|%4", _prefix, _key, _hiveCallID, _value];
```


**111**


Store value in database with the specified key


_Uses:  SET_

_Call Type: Asynchronous_



_Input: [Key](http://redis.io/topics/data-types-intro), Call Index ID, Value_

_Returns: Nothing_


```
"epochserver" callExtension format["111|%1:%2|%3|%4", _prefix, _key, _hiveCallID, _value];
```


**120**


_Uses:  SETEX_

_Call Type: Asynchronous_



_Input: [Key](http://redis.io/topics/data-types-intro), Expires, Call Index ID, Value_

_Returns: Nothing_


```
"epochserver" callExtension format["120|%1:%2|%3|%4|%5", _prefix, _key, _expires, _hiveCallID, _value];
```


**121**


Store value in database with the specified key and expiration.


_Uses:  SETEX_

_Call Type: Asynchronous_



_Input: [Key](http://redis.io/topics/data-types-intro), Expires, Call Index ID, Value_

_Returns: Nothing_

```
"epochserver" callExtension format["121|%1:%2|%3|%4|%5", _prefix, _key, _expires, _hiveCallID, _value];
```


**130**


Set a timeout on key. After the timeout has expired, the key will automatically be deleted.


_Uses:  [EXPIRE](http://redis.io/commands/expire)_

_Call Type: Synchronous_



_Input: [Key](http://redis.io/topics/data-types-intro), Expires_

_Returns: Nothing_

```
"epochserver" callExtension format ["130|%1:%2|%3", _prefix, _key, _expires];
```

**131**


Set a timeout on key. After the timeout has expired, the key will automatically be deleted.



_Uses:  [EXPIRE](http://redis.io/commands/expire)_

_Call Type: Asynchronous_



_Input: [Key](http://redis.io/topics/data-types-intro), Expires_

_Returns: Nothing_

```
"epochserver" callExtension format ["130|%1:%2|%3", _prefix, _key, _expires];
```


**140**

Sets or clears the bit at offset in the string value stored at key.



_Uses: [SETBIT](http://redis.io/commands/setbit)_

_Call Type: Synchronous_



_Input: [Key](http://redis.io/topics/data-types-intro), Bit Index, Value_

_Returns: Nothing_


```
"epochserver" callExtension format["140|%1:%2|%3|%4", _prefix, _key, _bitIndex, _value];
```


**141**


Sets or clears the bit at offset in the string value stored at key.


_Uses: [SETBIT](http://redis.io/commands/setbit)_

_Call Type: Asynchronous_



_Input: [Key](http://redis.io/topics/data-types-intro), Bit Index, Value_

_Returns: Nothing_


```
"epochserver" callExtension format["141|%1:%2|%3|%4", _prefix, _key, _bitIndex, _value];
```


**200**

_Uses: GET_

_Call Type: Synchronous_

_Input: [Key](http://redis.io/topics/data-types-intro), Bit Index, Value_

_Returns: _Array


```
_hiveResponse = "epochserver" callExtension format ["200|%1:%2", _prefix, _key];
```


**210**


_Uses: GET, TLL_


_Call Type: Synchronous_


_Input: [Key](http://redis.io/topics/data-types-intro), Bit Index, Value_

_Returns: Array_

```
_hiveResponse = "epochserver" callExtension format ["210|%1:%2", _prefix, _key];
```


**220**


_Uses: GETRANGE_


_Call Type: Synchronous_


_Input: [Key](http://redis.io/topics/data-types-intro), Start Index, Stop Index_

_Returns: Array_

```
_hiveResponse = "epochserver" callExtension format["220|%1:%2|%3|%4", _prefix, _key, _currentIndex, (_currentIndexMax-1)];
```


**230**

_Uses: GETBIT_

_Call Type: Synchronous_

_Input: [Key](http://redis.io/topics/data-types-intro), Bit Index, Value (0-1)_

_Returns: BOOL_


```
_hiveResponse = "epochserver" callExtension format["240|%1:%2|%3", _prefix, _key, _value];
```



**300**



_Uses: [TTL](http://redis.io/commands/ttl)_



_Call Type: Synchronous_



_Input: [Key](http://redis.io/topics/data-types-intro)_

_Returns: TTL in seconds_


```
"epochserver" callExtension format["300|%1:%2", _prefix, _uniqueID];
```


**400**

Removes the specified key.

_Uses: [DEL](http://redis.io/commands/del)_

_Call Type: Synchronous_



_Input: [Key](http://redis.io/topics/data-types-intro)_

_Returns: Nothing_


```
"epochserver" callExtension format["400|%1:%2", _prefix, _uniqueID];
```


**500**

_Uses: [PING](http://redis.io/commands/ping)_

_Call Type: Synchronous_

_Input: Nothing_

_Returns: "PONG"_

```
'epochserver' callExtension '500'
```



**501**

Get current time.



_Call Type: Synchronous_

_Input: Nothing_

_Returns: Array [YYYY,MM,DD,HH,MM,SS]_


```
'epochserver' callExtension '510'
```


**600**

Used to push data from database server in a queue.

_Uses: [LPOP](http://redis.io/commands/lpop) with CMD- prefix_



_Call Type: Synchronous_

_Input: Server InstanceID_

_Returns: Array [1,""]_


```
_response = "epochserver" callExtension format["600|%1",_instanceID];
```


**700**

Log in Redis



_Uses: LPUSH _with -LOG postfix



_Call Type: Synchronous_

_Input: Key Prefix, Message_

_Returns: Nothing_


```
"epochserver" callExtension format["700|%1|%2", _prefix, _message];
```


**701**

Log in Redis



_Uses: LPUSH _with -LOG postfix



_Call Type: Synchronous_

_Input: Key Prefix, Message_

_Returns: Nothing_



```
"epochserver" callExtension format["701|%1|%2", _prefix, _message];
```


**800**



Append strings specified to end of first line of publicvariable.txt and then runs BE command #loadEvents.



_Call Type: Synchronous_

_Input: Strings_

_Returns: Nothing_



```
"epochserver" callExtension format["800|%1|%2|%3|%4", _var1, _var2, _var3, _var4];
```


**801**

Append strings specified to end of first line of publicvariable.txt and then runs BE command #loadEvents.



_Call Type: Asynchronous_

_Input: Strings_

_Returns: Nothing_



```
"epochserver" callExtension format["800|%1|%2|%3|%4", _var1, _var2, _var3, _var4];
```

**810**

Get random string, [a-zA-Z]{5-10}, if only one string is requested it will return a string instead of a array



_Call Type: Synchronous_

_Input: Count_

_Returns: Array or String_





```
_response = 'epochserver' callExtension format['810|%1',_count];
```



**820**

Add ban to bans.txt and execute #loadBans (Depreciated should use 900 calls)



_Call Type: Synchronous_

_Input: Steam64ID, Reason_

_Returns: Nothing_



```
'epochserver' callExtension format['820|%1|%2',getPlayerUID _playerObj,_reason];
```

**821**

Add ban to bans.txt and execute #loadBans (Depreciated should use 900 calls)



_Call Type: Synchronous_

_Input: Steam64ID, Reason_

_Returns: Nothing_



```
epochserver' callExtension format['821|%1|%2',getPlayerUID _playerObj,_reason];
```


**901**

Broadcast message to server

_Call Type: Asynchronous_

_Input: Message_

_Returns: Nothing_

```
'epochserver' callExtension format['901|%1', _message];
```


**911**

Kick with message



_Call Type: Asynchronous_

_Input: player64ID, Message_

_Returns: Nothing_



```
'epochserver' callExtension format['911|%1|%2', _playerUID, _reason];
```



**921**

Ban with message and duration



_Call Type: Asynchronous_

_Input: player64ID, Message, duration_

_Returns: Nothing_


```
'epochserver' callExtension format['921|%1|%2|%3', _playerUID, _message, _duration];
```


**930**

Unlock server using BE command #unlock



_Call Type: Asynchronous_

_Input: Nothing_

_Returns: Nothing_



```
'epochserver' callExtension '930'
```

**931**

Lock server using BE command #lock



_Call Type: Asynchronous_

_Input: Nothing_

_Returns: Nothing_


```
'epochserver' callExtension '931'
```


**991**

Shutdown Server using BE #shutdown command.  

_Call Type: Asynchronous_

_Input: Nothing_

_Returns: Nothing_



```
'epochserver' callExtension '991'
```
