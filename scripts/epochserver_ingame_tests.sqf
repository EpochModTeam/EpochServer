// EpochServer in-game SQF test harness
//
// Usage from the Arma debug console:
//     [] execVM "@EpochServerTest\epochserver_ingame_tests.sqf";
//
// Or paste the whole file into the debug console and execute it.
//
// Results are written to the RPT with the [EpochServerSQFTest] prefix.

[] spawn {
    private _ext = "epochserver";
    private _prefix = format ["EPOCHSQFTEST:%1:%2", clientOwner, floor diag_tickTime];
    private _passed = 0;
    private _failed = 0;
    private _results = [];

    private _log = {
        params ["_message"];
        diag_log format ["[EpochServerSQFTest] %1", _message];
    };

    private _contains = {
        params ["_haystack", "_needle"];
        (_haystack find _needle) >= 0
    };

    private _record = {
        params ["_name", "_ok", "_command", "_output", ["_note", ""]];

        if (_ok) then {
            _passed = _passed + 1;
        } else {
            _failed = _failed + 1;
        };

        private _status = if (_ok) then { "PASS" } else { "FAIL" };
        private _line = format ["%1 %2 | %3 -> %4 %5", _status, _name, _command, _output, _note];
        _results pushBack _line;
        [_line] call _log;
    };

    private _call = {
        params ["_command"];
        _ext callExtension _command
    };

    ["========== EpochServer SQF Test Start =========="] call _log;
    [format ["Using key prefix: %1", _prefix]] call _log;
    systemChat "EpochServer SQF tests started. Watch the RPT for details.";

    private _out = "";
    private _cmd = "";
    private _ok = false;

    _cmd = "000";
    _out = [_cmd] call _call;
    _ok = ([_out, "["] call _contains) && (([_out, "TEST"] call _contains) || ([_out, "NA"] call _contains));
    ["000 config", _ok, _cmd, _out] call _record;

    _cmd = "500";
    _out = [_cmd] call _call;
    _ok = [_out, "PONG"] call _contains;
    ["500 redis ping", _ok, _cmd, _out] call _record;

    _cmd = "510";
    _out = [_cmd] call _call;
    _ok = [_out, "["] call _contains;
    ["510 current time", _ok, _cmd, _out] call _record;

    _cmd = "810|3";
    _out = [_cmd] call _call;
    _ok = [_out, "["] call _contains;
    ["810 random strings", _ok, _cmd, _out] call _record;

    _cmd = "840|hello";
    _out = [_cmd] call _call;
    _ok = [_out, "5d41402abc4b2a76b9719d911017c592"] call _contains;
    ["840 md5", _ok, _cmd, _out] call _record;

    private _key = format ["%1:SETGET", _prefix];
    private _value = '["arma","redis","ok",123,true]';

    _cmd = format ["110|%1|0|%2", _key, _value];
    _out = [_cmd] call _call;
    _ok = [_out, "OK"] call _contains;
    ["110 set", _ok, _cmd, _out] call _record;

    _cmd = format ["250|%1", _key];
    _out = [_cmd] call _call;
    _ok = [_out, '"1"'] call _contains;
    ["250 exists", _ok, _cmd, _out] call _record;

    _cmd = format ["200|%1", _key];
    _out = [_cmd] call _call;
    _ok = ([_out, "arma"] call _contains) && ([_out, "redis"] call _contains);
    ["200 get", _ok, _cmd, _out] call _record;

    _cmd = format ["220|%1|0|24", _key];
    _out = [_cmd] call _call;
    _ok = [_out, "arma"] call _contains;
    ["220 getrange", _ok, _cmd, _out] call _record;

    private _ttlKey = format ["%1:SETEX", _prefix];
    _cmd = format ["120|%1|60|0|%2", _ttlKey, '["temporary","value"]'];
    _out = [_cmd] call _call;
    _ok = [_out, "OK"] call _contains;
    ["120 setex", _ok, _cmd, _out] call _record;

    _cmd = format ["300|%1", _ttlKey];
    _out = [_cmd] call _call;
    _ok = !([_out, "[0]"] call _contains) && !([_out, '"-2"'] call _contains);
    ["300 ttl", _ok, _cmd, _out] call _record;

    _cmd = format ["210|%1", _ttlKey];
    _out = [_cmd] call _call;
    _ok = [_out, "temporary"] call _contains;
    ["210 get ttl", _ok, _cmd, _out] call _record;

    private _expireKey = format ["%1:EXPIRE", _prefix];
    _cmd = format ["110|%1|0|%2", _expireKey, '["expire","soon"]'];
    _out = [_cmd] call _call;
    _ok = [_out, "OK"] call _contains;
    ["110 set expire fixture", _ok, _cmd, _out] call _record;

    _cmd = format ["130|%1|2", _expireKey];
    _out = [_cmd] call _call;
    _ok = [_out, '"1"'] call _contains;
    ["130 expire", _ok, _cmd, _out] call _record;

    sleep 3;

    _cmd = format ["250|%1", _expireKey];
    _out = [_cmd] call _call;
    _ok = [_out, '"0"'] call _contains;
    ["250 exists after expire", _ok, _cmd, _out] call _record;

    private _bitKey = format ["%1:BITS", _prefix];
    _cmd = format ["140|%1|5|1", _bitKey];
    _out = [_cmd] call _call;
    _ok = [_out, "[1"] call _contains;
    ["140 setbit", _ok, _cmd, _out] call _record;

    _cmd = format ["240|%1|5", _bitKey];
    _out = [_cmd] call _call;
    _ok = [_out, '"1"'] call _contains;
    ["240 getbit", _ok, _cmd, _out] call _record;

    _cmd = format ["700|SQFTEST|In-game Redis log test %1", _prefix];
    _out = [_cmd] call _call;
    _ok = [_out, "[1"] call _contains;
    ["700 log", _ok, _cmd, _out] call _record;

    _cmd = "830";
    _out = [_cmd] call _call;
    _ok = [_out, "[1"] call _contains;
    ["830 incr", _ok, _cmd, _out] call _record;

    private _badKey = format ["%1:BAD", _prefix];
    _cmd = format ["110|%1|0|%2", _badKey, '"not an array"'];
    _out = [_cmd] call _call;
    _ok = [_out, "[0]"] call _contains;
    ["110 abuse reject", _ok, _cmd, _out] call _record;

    private _large = "";
    for "_i" from 1 to 12000 do {
        _large = _large + "x";
    };

    private _largeKey = format ["%1:LARGE", _prefix];
    _cmd = format ["110|%1|0|[""large"",""%2""]", _largeKey, _large];
    _out = [_cmd] call _call;
    _ok = [_out, "OK"] call _contains;
    ["110 set large fixture", _ok, "110|<large>", _out] call _record;

    _cmd = format ["200|%1", _largeKey];
    _out = [_cmd] call _call;
    _ok = ([_out, "[2"] call _contains) || ([_out, "[1"] call _contains);
    ["200 large page 1", _ok, _cmd, format ["len=%1 out=%2", count _out, _out select [0, ((count _out) min 80)]]] call _record;

    _cmd = format ["200|%1", _largeKey];
    _out = [_cmd] call _call;
    _ok = ([_out, "[2"] call _contains) || ([_out, "[1"] call _contains);
    ["200 large page 2", _ok, _cmd, format ["len=%1 out=%2", count _out, _out select [0, ((count _out) min 80)]]] call _record;

    _cmd = format ["600|%1", "testqueue"];
    _out = [_cmd] call _call;
    _ok = !([toLower _out, "panic"] call _contains);
    ["600 lpop smoke", _ok, _cmd, _out, "(requires external CMD:testqueue list data for a positive pop)"] call _record;

    {
        _cmd = format ["400|%1", _x];
        _out = [_cmd] call _call;
        [format ["cleanup %1 -> %2", _x, _out]] call _log;
    } forEach [_key, _ttlKey, _expireKey, _bitKey, _badKey, _largeKey];

    ["========== EpochServer SQF Test Results =========="] call _log;
    {
        [_x] call _log;
    } forEach _results;

    private _summary = format ["EpochServer SQF tests complete: %1 passed, %2 failed", _passed, _failed];
    [_summary] call _log;
    systemChat _summary;

    if (_failed > 0) then {
        systemChat "Some EpochServer SQF tests failed. Check the RPT for [EpochServerSQFTest].";
    };
};
