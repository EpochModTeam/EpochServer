#include <string>
#include "Logger.hpp"

#ifndef __EPOCHLIB_DEF__
#define __EPOCHLIB_DEF__

typedef unsigned char uint8;
typedef long long int int64;

struct EpochlibConfigRedis {
	std::string ip;
	unsigned short int port;
	std::string password;
	unsigned int dbIndex;
	Logger *logger;
};

struct EpochlibConfigSteamAPI {
	short int logging;
	std::string key;
	bool vacBanned;
	int vacMinNumberOfBans;
	int vacMaxDaysSinceLastBan;
	int playerAllowOlderThan;
};

struct EpochlibConfigBattlEye {
	std::string        ip;
	unsigned short int port;
	std::string        password;
	std::string        path;
};

struct EpochlibConfig {
	std::string battlEyePath;
	EpochlibConfigBattlEye battlEye;
	EpochlibConfigRedis redis;
	std::string hivePath;
	std::string profilePath;
	size_t outputSize;
	EpochlibConfigSteamAPI steamAPI;
	std::string instanceId;
	short int logAbuse;
};

struct EpochlibRedisExecute {
	bool success;
	std::string message;
};

#endif