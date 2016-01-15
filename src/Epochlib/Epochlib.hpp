/* Default Epochlib defines */
#include "defines.hpp"

#include "Logger.hpp"
#include "RedisConnector.hpp"
#include "SQF.hpp"
#include "../SteamAPI/SteamAPI.hpp"
#include "../external/md5.hpp"
#include "../external/ConfigFile.hpp"
#include <string>
#include <sstream>
#include <fstream>
#include <ctime>
#include <mutex>
#include <regex>
#include <pcre.h>

#ifndef __EPOCHLIB_H__
#define __EPOCHLIB_H__

#define EPOCHLIB_SQF_NOTHING 0
#define EPOCHLIB_SQF_STRING 1
#define EPOCHLIB_SQF_ARRAY 2

#define EPOCHLIB_SQF_RET_FAIL 0
#define EPOCHLIB_SQF_RET_SUCESS 1
#define EPOCHLIB_SQF_RET_CONTINUE 2

#define EPOCHLIB_SERVERMD5 "8497e70dafab88ea432338fee8c86579" //"426a526f91eea855dc889d21205e574c"

/* !!! TESTING DO NOT ENABLE IN PRODUCTION !!! */
//#define EPOCHLIB_TEST
#define EPOCHLIB_DISABLE_OFFICALCHECK

class Epochlib {
private:
	bool initialized;
	EpochlibConfig config;

	Logger *logger;
	RedisConnector *redis;

	bool _fileExist(std::string FileName);
	bool _loadConfig(std::string ConfigFilename);
	std::string _getBattlEyeGUID(int64 SteamId);
	SQF _redisExecToSQF(EpochlibRedisExecute RedisExecute, int ForceMessageOutputType);
	bool _isOffialServer();

	pcre *setValueRegex;

	EpochlibRedisExecute tempGet;

	std::mutex tempSetMutex;
	std::string tempSet;

	std::mutex steamIdWhitelistMutex;
	std::vector<int64> steamIdWhitelist;

public:
	Epochlib(std::string ConfigPath, std::string ProfilePath, int OutputSize);
	~Epochlib();

	/* Get Config
	*/
	std::string getConfig();

	/* Initial player check
	* 64Bit Steam community id
	*/
	std::string initPlayerCheck(int64 SteamId);

	/* Add ban with reason to bans.txt
	* 64Bit Steam community id
	* Reason
	*/
	std::string addBan(int64 SteamId, std::string Reason);

	/* Add whitelisted string to publicvariable.txt
	* String needs to be whitelisted
	*/
	std::string updatePublicVariable(std::vector<std::string> WhitelistStrings);
	std::string getRandomString(int StringCount);

	/* Increase bancount
	*/
	std::string increaseBancount();

	/* Get current time
	*/
	std::string getCurrentTime();

	/* Redis GET
	* Key
	*/
	std::string get(std::string Key);
	std::string getRange(std::string Key, std::string Value, std::string Value2);
	std::string getTtl(std::string Key);
	std::string getbit(std::string Key, std::string Value);
	std::string exists(std::string Key);

	/* Redis SET / SETEX
	*/
	std::string setTemp(std::string Key, std::string Value, std::string Value2);
	std::string set(std::string Key, std::string Value, std::string Value2);
	std::string setex(std::string Key, std::string Value, std::string Value2, std::string Value3);
	std::string expire(std::string Key, std::string TTL);
	std::string setbit(std::string Key, std::string Value, std::string Value2);

	/* Redis DEL
	* Key
	*/
	std::string del(std::string Key);

	/* Redis PING
	*/
	std::string ping();

	/* Redis LPOP with a given prefix
	*/
	std::string lpopWithPrefix(std::string Prefix, std::string Key);

	/* Redis TTL
	* Key
	*/
	std::string ttl(std::string Key);

	std::string log(std::string Key, std::string Value);

	std::string getServerMD5();
	
	/* BE broadcast message 
	* Message
	*/
	void beBroadcastMessage (std::string Message);
	
	/* BE kick
	* PlayerUID
	* Message
	*/
	void beKick (std::string PlayerUID, std::string Message);
	
	/* BE ban
	* PlayerUID
	* Message
	* Duration (minutes)
	*/
	void beBan(std::string PlayerUID, std::string Message, std::string Duration);
	
	/* BE shutdown 
	*/
	void beShutdown();
	
	/* BE lock / unlock
	*/
	void beLock();
	void beUnlock();
};

#endif