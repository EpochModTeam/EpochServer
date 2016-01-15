#include "Epochlib.hpp"
#include "../SteamAPI/SteamAPI.hpp"
#include "../BattlEye/BEClient.hpp"
#include <cstdlib>
#include <ctime>
#include <algorithm>
#include <iomanip>

#ifdef WIN32
#pragma comment(lib, "pcre3.lib")
#endif


Epochlib::Epochlib(std::string _configPath, std::string _profilePath, int _outputSize) {
	this->initialized = false;
	this->config.hivePath.assign(_configPath);
	this->config.profilePath.assign(_profilePath);
	this->config.outputSize = _outputSize;

	// Init random
	std::srand(std::time(0));

	this->logger = new Logger(this->config.profilePath + "/EpochServer.log");

#ifndef EPOCHLIB_TEST
#ifndef EPOCHLIB_DISABLE_OFFICALCHECK
	/* Log & exit if server does not use official server files */
	if (!this->_isOffialServer()) {
		this->logger->log("Wrong server files");
		exit(1);
	}
#endif
#endif

	if (this->_loadConfig(_configPath + "/EpochServer.ini") || this->_loadConfig(_profilePath + "/EpochServer.ini") || 
	    this->_loadConfig(_configPath + "/epochserver.ini")) {
		this->initialized = true;
	}
	else {
		this->logger->log("EpochServer.ini not found in (" + _configPath + ", " + _profilePath + ")");
		exit(1);
	}

#ifndef EPOCHLIB_TEST
	this->redis = new RedisConnector(this->config.redis);
#endif

	// Setup regex validation
	const char *error;
	int erroffset;
	std::stringstream regexString;
	regexString << "(?(DEFINE)";
	regexString << "(?<boolean>true|false)";
	regexString << "(?<number>-?(?=[1-9]|0(?!\\d))\\d+(\\.\\d+)?([eE][+\\-]?\\d+)?)";
	regexString << "(?<string>\"([^\"]*|\\\\\\\\[\"\\\\\\\\bfnrt\\/]|\\\\u[0-9a-f]{4})*\")";
	regexString << "(?<array>\\[(?:(?&container)(?:,(?&container))*)?\\s*\\])";
	regexString << "(?<container>\\s*(?:(?&boolean)|(?&number)|(?&string)|(?&array))\\s*)";
	regexString << ")";
	regexString << "\\A(?&array)\\Z";
	this->setValueRegex = pcre_compile(regexString.str().c_str(), PCRE_CASELESS | PCRE_DOTALL | PCRE_EXTENDED, &error, &erroffset, NULL);
	if (this->setValueRegex == NULL){
		this->logger->log("PCRE compile error: " + std::string(error, erroffset));
		exit(1);
	}

	this->tempGet.success = 0;
}
Epochlib::~Epochlib() {

}

std::string Epochlib::getConfig() {
	SQF returnSqf;

	returnSqf.push_str(this->config.instanceId.c_str());
	returnSqf.push_number(this->config.steamAPI.key.empty() ? 0 : 1);

	return returnSqf.toArray();
}

std::string Epochlib::initPlayerCheck(int64 _steamId) {
	bool proceeded = false;

	// Not in whitelist
	if (std::find(this->steamIdWhitelist.begin(), this->steamIdWhitelist.end(), _steamId) == this->steamIdWhitelist.end()) {

		// SteamAPI key is not empty
		if (!this->config.steamAPI.key.empty()) {
			SteamAPI steamAPI(this->config.steamAPI.key);

			rapidjson::Document document;
			std::stringstream steamIds;
			steamIds << _steamId;

			// VAC check
			if (!proceeded && steamAPI.GetPlayerBans(steamIds.str(), &document)) {
				if (this->config.steamAPI.logging >= 2) {
					std::stringstream log;
					log << "[SteamAPI] VAC check " << _steamId << std::endl;
					log << "- VACBanned: " << (document["players"][0]["VACBanned"].GetBool() ? "true" : "false") << std::endl;
					log << "- DaysSinceLastBan: " << document["players"][0]["DaysSinceLastBan"].GetInt() << std::endl;
					log << "- NumberOfVACBans: " << document["players"][0]["NumberOfVACBans"].GetInt();
					this->logger->log(log.str());
				}

				if (!proceeded && this->config.steamAPI.vacBanned && document["players"][0]["VACBanned"].GetBool()) {
					if (this->config.steamAPI.logging >= 1) {
						std::stringstream log;
						log << "[SteamAPI] VAC ban " << _steamId << std::endl;
						log << "- VACBanned: " << (document["players"][0]["VACBanned"].GetBool() ? "true" : "false");
						this->logger->log(log.str());
					}

					this->addBan(_steamId, "VAC Ban");
					proceeded = true;
				}
				if (!proceeded && this->config.steamAPI.vacMaxDaysSinceLastBan > 0 && document["players"][0]["DaysSinceLastBan"].GetInt() < this->config.steamAPI.vacMaxDaysSinceLastBan) {
					if (this->config.steamAPI.logging >= 1) {
						std::stringstream log;
						log << "[SteamAPI] VAC ban " << _steamId << std::endl;
						log << "- DaysSinceLastBan: " << document["players"][0]["DaysSinceLastBan"].GetInt();
						this->logger->log(log.str());
					}

					this->addBan(_steamId, "VAC Ban");
					proceeded = true;
				}
				if (!proceeded && this->config.steamAPI.vacMinNumberOfBans > 0 && document["players"][0]["NumberOfVACBans"].GetInt() >= this->config.steamAPI.vacMinNumberOfBans) {
					if (this->config.steamAPI.logging >= 1) {
						std::stringstream log;
						log << "[SteamAPI] VAC ban " << _steamId << std::endl;
						log << "- NumberOfVACBans: " << document["players"][0]["NumberOfVACBans"].GetInt();
						this->logger->log(log.str());
					}

					this->addBan(_steamId, "VAC Ban");
					proceeded = true;
				}
			}

			// Player check
			if (!proceeded && steamAPI.GetPlayerSummaries(steamIds.str(), &document)) {
				if (this->config.steamAPI.logging >= 2) {
					std::stringstream log;
					log << "[SteamAPI] Player check " << _steamId << std::endl;
					log << "- timecreated: " << document["response"]["players"][0]["timecreated"].GetInt();
					this->logger->log(log.str());
				}

				if (!proceeded && this->config.steamAPI.playerAllowOlderThan > 0) {
					std::time_t currentTime = std::time(nullptr);

					if ((currentTime - document["response"]["players"][0]["timecreated"].GetInt()) < this->config.steamAPI.playerAllowOlderThan) {
						if (this->config.steamAPI.logging >= 1) {
							std::stringstream log;
							log << "[SteamAPI] Player ban " << _steamId << std::endl;
							log << "- timecreated: " << document["response"]["players"][0]["timecreated"].GetInt() << std::endl;
							log << "- current: " << currentTime;
							this->logger->log(log.str());
						}

						this->addBan(_steamId, "New account filter");
						proceeded = true;
					}
				}
			}
		}

		// Not proceeded -> fine
		if (!proceeded) {
			this->steamIdWhitelistMutex.lock();
			this->steamIdWhitelist.push_back(_steamId);
			this->steamIdWhitelistMutex.unlock();
		}
	}

	return "";
}

std::string Epochlib::addBan(int64 _steamId, std::string _reason) {
	std::string battleyeGUID = this->_getBattlEyeGUID(_steamId);
	std::string bansFilename = this->config.battlEyePath + "/bans.txt";
	SQF returnSQF;

	std::ofstream bansFile;
	bansFile.open(bansFilename, std::ios::app);
	if (bansFile.good()) {
		bansFile << battleyeGUID << " -1 " << _reason << std::endl;
		bansFile.close();

                this->logger->log("BEClient: try to connect " + this->config.battlEye.ip);
                BEClient bec     (this->config.battlEye.ip.c_str(), this->config.battlEye.port);
                bec.sendLogin    (this->config.battlEye.password.c_str());
                bec.readResponse (BE_LOGIN);
                if (bec.isLoggedIn()) {
                    this->logger->log("BEClient: logged in!");
                    bec.sendCommand  ("loadBans");
                    bec.readResponse (BE_COMMAND);
                
                    bec.sendCommand  ("players");
                    bec.readResponse (BE_COMMAND);
                        
                    int playerSlot = bec.getPlayerSlot (battleyeGUID);
                    if (playerSlot >= 0) {
                        std::stringstream ss;
                        ss << "ban " << playerSlot << " 0 " << _reason;
                        bec.sendCommand  (ss.str().c_str());
                        bec.readResponse (BE_COMMAND);
                    }
                } else {
                    this->logger->log("BEClient: login failed!");
                }
                bec.disconnect   ();

		returnSQF.push_str("1");
		returnSQF.push_str(battleyeGUID.c_str());
	}
	else {
		bansFile.close();
		returnSQF.push_str("0");
	}

	return returnSQF.toArray();
}

std::string Epochlib::updatePublicVariable(std::vector<std::string> _whitelistStrings) {
	std::string pvFilename = this->config.battlEyePath + "/publicvariable.txt";
	std::string pvContent = "";
	bool pvFileOriginalFound = false;
	SQF returnSQF;

	// Try to read the original file content
	std::ifstream pvFileOriginal(pvFilename + ".original");
	if (pvFileOriginal.good()) {
		// Jump to the end
		pvFileOriginal.seekg(0, std::ios::end);
		// Allocate memory
		pvContent.reserve((unsigned int)pvFileOriginal.tellg());
		// Jump to the begin
		pvFileOriginal.seekg(0, std::ios::beg);
		// Assing content
		pvContent.assign(std::istreambuf_iterator<char>(pvFileOriginal), std::istreambuf_iterator<char>());

		pvFileOriginalFound = true;
	}
	pvFileOriginal.close();

	// Original file not found
	if (!pvFileOriginalFound) {
		std::ifstream pvFileCurrent(pvFilename);
		if (pvFileCurrent.good()) {
			// Jump to the end
			pvFileCurrent.seekg(0, std::ios::end);
			// Allocate memory
			pvContent.reserve((unsigned int)pvFileCurrent.tellg());
			// Jump to the begin
			pvFileCurrent.seekg(0, std::ios::beg);
			// Assing content
			pvContent.assign(std::string(std::istreambuf_iterator<char>(pvFileCurrent), std::istreambuf_iterator<char>()));

			pvFileCurrent.close();
		}
		else {
			pvFileCurrent.close();
			returnSQF.push_str("0");
			this->logger->log("publicvariable.txt not found in " + this->config.battlEyePath);
			return returnSQF.toArray();
		}

		// Copy content to the original file
		std::ofstream pvFileOriginalNew(pvFilename + ".original");
		if (pvFileOriginalNew.good()) {
			pvFileOriginalNew << pvContent;
			pvFileOriginalNew.close();
		}
		else {
			pvFileOriginalNew.close();
			returnSQF.push_str("0");
			return returnSQF.toArray();
		}
	}

	// write new pvFile
	std::ofstream pvFileNew(pvFilename);
	if (pvFileNew.good()) {
		pvFileNew << pvContent;

		for (std::vector<std::string>::iterator
			it = _whitelistStrings.begin();
			it != _whitelistStrings.end();
		) {
			pvFileNew << " !=\"" << *it << "\"";
			it++;
		}

		returnSQF.push_str("1");
	}
	else {
		returnSQF.push_str("0");
	}
	pvFileNew.close();
	
	this->logger->log("BEClient: try to connect " + this->config.battlEye.ip);
	BEClient bec     (this->config.battlEye.ip.c_str(), this->config.battlEye.port);
	bec.sendLogin    (this->config.battlEye.password.c_str());
	bec.readResponse (BE_LOGIN);
	if (bec.isLoggedIn()) {
	    this->logger->log("BEClient: logged in!");
	    bec.sendCommand  ("loadEvents");
	    bec.readResponse (BE_COMMAND);
	} else {
	    this->logger->log("BEClient: login failed!");
	}
	bec.disconnect   ();

	return returnSQF.toArray();
}
std::string Epochlib::getRandomString(int _stringCount) {
	SQF returnSQF;
	std::vector<std::string> randomStrings;

	// Define char pool
	const char charPool[] = "abcdefghijklmnopqrstuvwxyz";
	int charPoolSize = sizeof(charPool) - 1;

	for (int stringCounter = 0; stringCounter < _stringCount; stringCounter++) {
		std::string randomString;
		int stringLength = (std::rand() % 5) + 5; //random string size between 5-10

		for (int i = 0; i < stringLength; i++) {
			randomString += charPool[std::rand() % charPoolSize];
		}

		// Build unique string array
		if (std::find(randomStrings.begin(), randomStrings.end(), randomString) == randomStrings.end() && randomString.find("god") == std::string::npos) {
			randomStrings.push_back(randomString);
		}
		else {
			stringCounter--;
		}
	}

	if (_stringCount == 1 && randomStrings.size() == 1) {
		return randomStrings.at(0);
	}
	else {
		for (std::vector<std::string>::iterator it = randomStrings.begin(); it != randomStrings.end(); ++it) {
			returnSQF.push_str(it->c_str());
		}

		return returnSQF.toArray();
	}
}

std::string Epochlib::increaseBancount() {
	return this->_redisExecToSQF(this->redis->execute("INCR %s", "ahb-cnt"), EPOCHLIB_SQF_STRING).toArray();
}

std::string Epochlib::getCurrentTime() {
	SQF returnSQF;
	char buffer[8];
	size_t bufferSize;

	time_t t = time(0);
	struct tm * currentTime = localtime(&t);

	bufferSize = strftime(buffer, 8, "%Y", currentTime);
	returnSQF.push_number(buffer, bufferSize);

	bufferSize = strftime(buffer, 8, "%m", currentTime);
	returnSQF.push_number(buffer, bufferSize);

	bufferSize = strftime(buffer, 8, "%d", currentTime);
	returnSQF.push_number(buffer, bufferSize);

	bufferSize = strftime(buffer, 8, "%H", currentTime);
	returnSQF.push_number(buffer, bufferSize);

	bufferSize = strftime(buffer, 8, "%M", currentTime);
	returnSQF.push_number(buffer, bufferSize);

	bufferSize = strftime(buffer, 8, "%S", currentTime);
	returnSQF.push_number(buffer, bufferSize);

	return returnSQF.toArray();
}


std::string Epochlib::getRange(std::string _key, std::string _value, std::string _value2) {
	SQF returnSqf;
	EpochlibRedisExecute value = this->redis->execute("GETRANGE %s %s %s", _key.c_str(), _value.c_str(), _value2.c_str());
	if (value.success == 1) {
		returnSqf.push_number(EPOCHLIB_SQF_RET_SUCESS);
		std::string output = value.message;
		std::stringstream outputSteam;
		for (std::string::iterator it = output.begin(); it != output.end(); ++it) {
			if (*it == '\'') {
				outputSteam << '\'';
			}
			outputSteam << *it;
		}
		returnSqf.push_str(outputSteam.str().c_str(), 1);
	} else {
		returnSqf.push_number(EPOCHLIB_SQF_RET_FAIL);
	}
	return returnSqf.toArray();
}

std::string Epochlib::get(std::string _key) {
	SQF returnSqf;


	// No temp GET found -> GET new one
	if (this->tempGet.success != 1) {
		this->tempGet = this->redis->execute("GET %s", _key.c_str());
	}

	// GET success proceed
	if (this->tempGet.success == 1) {
		size_t messageSize = 0;

		// Temp message > possible output
		if (this->tempGet.message.size() > this->config.outputSize) {
			returnSqf.push_number(EPOCHLIB_SQF_RET_CONTINUE);

			messageSize = this->config.outputSize - 20;
			std::string output = this->tempGet.message.substr(0, messageSize);
			std::stringstream outputSteam;
			for (std::string::iterator it = output.begin(); it != output.end(); ++it) {
				if (*it == '\'') {
					outputSteam << '\'';
				}
				outputSteam << *it;
			} 
			returnSqf.push_str(outputSteam.str().c_str(), 1);
		}
		// Message in one row possible
		else {
			returnSqf.push_number(EPOCHLIB_SQF_RET_SUCESS); // single row

			messageSize = this->tempGet.message.size();
			std::string output = this->tempGet.message;
			std::stringstream outputSteam;
			for (std::string::iterator it = output.begin(); it != output.end(); ++it) {
				if (*it == '\'') {
					outputSteam << '\'';
				}
				outputSteam << *it;
			}
			returnSqf.push_str(outputSteam.str().c_str(), 1);

			this->tempGet.success = 0;
		}

                if (this->tempGet.message.size() >= this->config.outputSize - 20)
		    this->tempGet.message.erase(this->tempGet.message.begin(), this->tempGet.message.begin() + this->config.outputSize - 20);
	}
	else {
		returnSqf.push_number(EPOCHLIB_SQF_RET_FAIL);
	}

	return returnSqf.toArray();
}

std::string Epochlib::getTtl(std::string _key) {
	SQF returnSqf;

	// No temp GET found -> GET new one
	if (this->tempGet.success != 1) {
		this->tempGet = this->redis->execute("GET %s", _key.c_str());

		if (this->tempGet.success == 1) {
			
			EpochlibRedisExecute ttl = this->redis->execute("TTL %s", _key.c_str());

			size_t messageSize = 0;

			// Temp message > possible output
			if (this->tempGet.message.size() > this->config.outputSize) {
				returnSqf.push_number(EPOCHLIB_SQF_RET_CONTINUE);

				if (ttl.success == 1) {
					returnSqf.push_number(std::atol(ttl.message.c_str()));
				}
				else {
					returnSqf.push_number(-1);
				}

				messageSize = this->config.outputSize - 20;
				std::string output = this->tempGet.message.substr(0, messageSize);
				std::stringstream outputSteam;
				for (std::string::iterator it = output.begin(); it != output.end(); ++it) {
					if (*it == '\'') {
						outputSteam << '\'';
					}
					outputSteam << *it;
				}
				returnSqf.push_str(outputSteam.str().c_str(), 1);
			}
			// Message in one row possible
			else {
				returnSqf.push_number(EPOCHLIB_SQF_RET_SUCESS); // single row

				
				if (ttl.success == 1) {
					returnSqf.push_number(std::atol(ttl.message.c_str()));
				}
				else {
					returnSqf.push_number(-1);
				}

				messageSize = this->tempGet.message.size();
				std::string output = this->tempGet.message;
				std::stringstream outputSteam;
				for (std::string::iterator it = output.begin(); it != output.end(); ++it) {
					if (*it == '\'') {
						outputSteam << '\'';
					}
					outputSteam << *it;
				}
				returnSqf.push_str(outputSteam.str().c_str(), 1);

				this->tempGet.success = 0;
			}
                        
                        if (this->tempGet.message.size() >= this->config.outputSize - 20)
			    this->tempGet.message.erase(this->tempGet.message.begin(), this->tempGet.message.begin() + this->config.outputSize - 20);
		}
		else {
			returnSqf.push_number(EPOCHLIB_SQF_RET_FAIL);
		}
	}
	else {
		returnSqf.push_number(EPOCHLIB_SQF_RET_FAIL);
	}

	return returnSqf.toArray();
}

std::string Epochlib::setTemp(std::string _key, std::string _value, std::string _value2) {
	
	// Append to temporarily setter
	/*
	this->tempSetMutex.lock();
	this->tempSet.append(_value);
	this->tempSetMutex.unlock();
	*/
	
	this->redis->execute("APPEND tmp-%s-%s %s", _value.c_str(), _key.c_str(), _value2.c_str());
	this->redis->execute("EXPIRE tmp-%s-%s 10", _value.c_str(), _key.c_str());

	SQF sqf;
	sqf.push_number(EPOCHLIB_SQF_RET_SUCESS);
	return sqf.toArray();
}

std::string Epochlib::set(std::string _key, std::string _value, std::string _value2) {

	// Combine temporarily setter & value
	/*
	this->tempSetMutex.lock();
	std::string value = this->tempSet + _value;
	this->tempSet.clear();
	this->tempSetMutex.unlock();
	*/

	EpochlibRedisExecute temp = this->redis->execute("GET tmp-%s-%s", _value.c_str(), _key.c_str());
	if (temp.message != "") {
		this->redis->execute("DEL tmp-%s-%s", _value.c_str(), _key.c_str());
	}
	std::string value = temp.message + _value2;

	int regexReturnCode = pcre_exec(this->setValueRegex, NULL, value.c_str(), value.length(), 0, 0, NULL, NULL);
	if (regexReturnCode == 0) {
		return this->_redisExecToSQF(this->redis->execute("SET %s %s", _key.c_str(), value.c_str()), EPOCHLIB_SQF_NOTHING).toArray();
	}
	else {
		if (this->config.logAbuse > 0) {
			this->logger->log("[Abuse] SETEX key " + _key + " does not match the allowed syntax!" + (this->config.logAbuse > 1 ? "\n" + value : ""));
		}

		SQF sqf;
		sqf.push_number(EPOCHLIB_SQF_RET_FAIL);
		return sqf.toArray();
	}
}

std::string Epochlib::setex(std::string _key, std::string _ttl, std::string _value2, std::string _value3) {

	// Combine temporarily setter & value
	/*
	this->tempSetMutex.lock();
	std::string value = this->tempSet + _value2;
	this->tempSet.clear();
	this->tempSetMutex.unlock();
	*/

	EpochlibRedisExecute temp = this->redis->execute("GET tmp-%s-%s", _value2.c_str(), _key.c_str());
	if (temp.message != "") {
		this->redis->execute("DEL tmp-%s-%s", _value2.c_str(), _key.c_str());
	}
	std::string value = temp.message + _value3;

	int regexReturnCode = pcre_exec(this->setValueRegex, NULL, value.c_str(), value.length(), 0, 0, NULL, NULL);
	if (regexReturnCode == 0) {
		return this->_redisExecToSQF(this->redis->execute("SETEX %s %s %s", _key.c_str(), _ttl.c_str(), value.c_str()), EPOCHLIB_SQF_NOTHING).toArray();
	}
	else {
		if (this->config.logAbuse > 0) {
			this->logger->log("[Abuse] SETEX key " + _key + " does not match the allowed syntax!" + (this->config.logAbuse > 1 ? "\n" + value : ""));
		}

		SQF sqf;
		sqf.push_number(EPOCHLIB_SQF_RET_FAIL);
		return sqf.toArray();
	}
}

std::string Epochlib::expire(std::string _key, std::string _ttl) {
	return this->_redisExecToSQF(this->redis->execute("EXPIRE %s %s", _key.c_str(), _ttl.c_str()), EPOCHLIB_SQF_NOTHING).toArray();
}

std::string Epochlib::setbit(std::string _key, std::string _value, std::string _value2) {
	return this->_redisExecToSQF(this->redis->execute("SETBIT %s %s %s", _key.c_str(), _value.c_str(), _value2.c_str()), EPOCHLIB_SQF_NOTHING).toArray();
}

std::string Epochlib::getbit(std::string _key, std::string _value) {
	return this->_redisExecToSQF(this->redis->execute("GETBIT %s %s", _key.c_str(), _value.c_str()), EPOCHLIB_SQF_STRING).toArray();
}

std::string Epochlib::exists(std::string _key) {
	return this->_redisExecToSQF(this->redis->execute("EXISTS %s", _key.c_str()), EPOCHLIB_SQF_STRING).toArray();
}

std::string Epochlib::del(std::string _key) {
	return this->_redisExecToSQF(this->redis->execute("DEL %s", _key.c_str()), EPOCHLIB_SQF_NOTHING).toArray();
}

std::string Epochlib::ping() {
	return this->_redisExecToSQF(this->redis->execute("PING"), EPOCHLIB_SQF_NOTHING).toArray();
}

std::string Epochlib::lpopWithPrefix(std::string _prefix, std::string _key) {
	return this->_redisExecToSQF(this->redis->execute("LPOP %s%s", _prefix.c_str(), _key.c_str()), EPOCHLIB_SQF_STRING).toArray();
}

std::string Epochlib::ttl(std::string _key) {
	return this->_redisExecToSQF(this->redis->execute("TTL %s", _key.c_str()), EPOCHLIB_SQF_STRING).toArray();
}

std::string Epochlib::log(std::string _key, std::string _value) {
	char formatedTime[64];
	time_t t = time(0);
	struct tm * currentTime = localtime(&t);

	strftime(formatedTime, 64, "%Y-%m-%d %H:%M:%S ", currentTime);

	return this->_redisExecToSQF(this->redis->execute("LPUSH %s-LOG %s%s", _key.c_str(), formatedTime, _value.c_str()), EPOCHLIB_SQF_NOTHING).toArray();
}

std::string Epochlib::getServerMD5() {
	std::string serverMD5;

	std::string addonPath = this->config.hivePath + "/addons/a3_epoch_server.pbo";
	FILE *srvFile = fopen(addonPath.c_str(), "rb");
	if (srvFile != NULL) {
		MD5 md5Context;
		int bytes;
		unsigned char data[1024];

		while ((bytes = fread(data, 1, 1024, srvFile)) != 0) {
			md5Context.update(data, bytes);
		}

		md5Context.finalize();
		serverMD5 = md5Context.hexdigest();
	}
	else {
		serverMD5 = addonPath;
	}
	
	return serverMD5;
}

std::string Epochlib::_getBattlEyeGUID(int64 _steamId) {
	uint8 i = 0;
	uint8 parts[8] = { 0 };

	do
	{
		parts[i++] = _steamId & 0xFF;
	} while (_steamId >>= 8);

	std::stringstream bestring;
	bestring << "BE";
	for (unsigned int i = 0; i < sizeof(parts); i++) {
		bestring << char(parts[i]);
	}

	MD5 md5 = MD5(bestring.str());

	return md5.hexdigest();
}

bool Epochlib::_fileExist(std::string _filename) {
	std::ifstream file(_filename.c_str());

	if (file.good()) {
		file.close();
		return true;
	}
	else {
		file.close();
		return false;
	}
}

bool Epochlib::_loadConfig(std::string configFilename) {
	if (this->_fileExist(configFilename)) {
		ConfigFile configFile(configFilename);

		// EpochServer config
		this->config.battlEyePath   = (std::string)configFile.Value("EpochServer", "BattlEyePath", (this->config.profilePath.length() > 0 ? this->config.profilePath + "/battleye" : ""));
		this->config.instanceId     = (std::string)configFile.Value("EpochServer", "InstanceID", "NA123");
		this->config.logAbuse       = (unsigned short int)configFile.Value("EpochServer", "LogAbuse", 0);
		
		this->config.battlEye.ip    = (std::string)configFile.Value("EpochServer", "IP", "127.0.0.1");
		this->config.battlEye.port  = (unsigned short int)configFile.Value("EpochServer", "Port", 2302);
		this->config.battlEye.password = (std::string)configFile.Value("EpochServer", "Password", "");
		this->config.battlEye.path  = this->config.battlEyePath;

		// Redis config
		this->config.redis.ip       = (std::string)configFile.Value("Redis", "IP", "127.0.0.1");
		this->config.redis.port     = (unsigned short int)configFile.Value("Redis", "Port", 6379);
		this->config.redis.password = (std::string)configFile.Value("Redis", "Password", "");
		this->config.redis.dbIndex  = (unsigned int)configFile.Value("Redis", "DB", 0);
		this->config.redis.logger   = this->logger;

		// SteamApi
		this->config.steamAPI.logging                = configFile.Value("SteamAPI", "Logging", 0);
		this->config.steamAPI.key                    = (std::string)configFile.Value("SteamAPI", "Key", "");
		this->config.steamAPI.vacBanned              = configFile.Value("SteamAPI", "VACBanned", 0) > 0;
		this->config.steamAPI.vacMinNumberOfBans     = configFile.Value("SteamAPI", "VACMinimumNumberOfBans", 0);
		this->config.steamAPI.vacMaxDaysSinceLastBan = configFile.Value("SteamAPI", "VACMaximumDaysSinceLastBan", 0);
		this->config.steamAPI.playerAllowOlderThan   = configFile.Value("SteamAPI", "PlayerAllowOlderThan", 0);

		return true;
	}
	else {
		return false;
	}
}

SQF Epochlib::_redisExecToSQF(EpochlibRedisExecute _redisExecute, int _forceMsgOutputType) {
	SQF returnSQF;

	returnSQF.push_number(_redisExecute.success ? EPOCHLIB_SQF_RET_SUCESS : EPOCHLIB_SQF_RET_FAIL);
	if (!_redisExecute.message.empty()) {
		if (_redisExecute.message.at(0) == '[') {
			returnSQF.push_array(_redisExecute.message);
		}
		else {
			returnSQF.push_str(_redisExecute.message.c_str());
		}
	}
	else if (_forceMsgOutputType >= 0) {
		if (_forceMsgOutputType == EPOCHLIB_SQF_STRING) {
			returnSQF.push_str("");
		}
		else if (_forceMsgOutputType == EPOCHLIB_SQF_ARRAY) {
			returnSQF.push_array("[]");
		}
	}

	return returnSQF;
}

bool Epochlib::_isOffialServer() {
	return this->getServerMD5() == EPOCHLIB_SERVERMD5 ? true : false;
}

// Battleye Integration

void Epochlib::beBroadcastMessage (std::string msg) {
	if (msg.empty()) 
	    return;

        this->logger->log("BEClient: try to connect " + this->config.battlEye.ip);
	BEClient bec     (this->config.battlEye.ip.c_str(), this->config.battlEye.port);	
	bec.sendLogin    (this->config.battlEye.password.c_str());
	bec.readResponse (BE_LOGIN);
	if (bec.isLoggedIn()) {
	    this->logger->log("BEClient: logged in!");
	    std::stringstream ss;
	    ss << "say -1 " << msg;
	    bec.sendCommand  (ss.str().c_str());
	    bec.readResponse (BE_COMMAND);
	} else {
	    this->logger->log("BEClient: login failed!");
	}
	bec.disconnect   ();
}

void Epochlib::beKick (std::string playerUID, std::string msg) {
	if (playerUID.empty() || msg.empty()) 
	    return;
	
	this->logger->log("BEClient: try to connect " + this->config.battlEye.ip);
	BEClient bec     (this->config.battlEye.ip.c_str(), this->config.battlEye.port);
	bec.sendLogin    (this->config.battlEye.password.c_str());
	bec.readResponse (BE_LOGIN);
	if (bec.isLoggedIn()) {
	    this->logger->log("BEClient: logged in!");
	    bec.sendCommand  ("players");
	    bec.readResponse (BE_COMMAND);
        
	    std::string playerGUID = this->_getBattlEyeGUID (atoll(playerUID.c_str()));
	    int playerSlot         = bec.getPlayerSlot (playerGUID);
	    if (playerSlot < 0) {
	        bec.disconnect();
                return;
            }
	
	    std::stringstream ss;
	    ss << "kick " << playerSlot << ' ' << msg;
	    bec.sendCommand  (ss.str().c_str());
	    bec.readResponse (BE_COMMAND);
        } else {
            this->logger->log("BEClient: login failed!");
        }
	bec.disconnect   ();
}

void Epochlib::beBan(std::string playerUID, std::string msg, std::string duration) {
    if (playerUID.empty() || msg.empty())
        return;
            
    if (duration.empty())
        duration = "-1";

    this->logger->log("BEClient: try to connect " + this->config.battlEye.ip);
    BEClient bec     (this->config.battlEye.ip.c_str(), this->config.battlEye.port);
    bec.sendLogin    (this->config.battlEye.password.c_str());
    bec.readResponse (BE_LOGIN);
    if (bec.isLoggedIn()) {
        this->logger->log("BEClient: logged in!");
        bec.sendCommand  ("players");
        bec.readResponse (BE_COMMAND);

        std::string playerGUID = this->_getBattlEyeGUID (atoll(playerUID.c_str()));
        int playerSlot         = bec.getPlayerSlot (playerGUID);
        if (playerSlot < 0) {
            bec.disconnect();
            return;
        }
        
        std::stringstream ss;
        ss << "ban " << playerSlot << ' ' << duration << ' ' << msg;
        bec.sendCommand  (ss.str().c_str());
        bec.readResponse (BE_COMMAND);
    } else {
        this->logger->log("BEClient: login failed!");
    }
    bec.disconnect   ();
}

void Epochlib::beShutdown() {
    this->logger->log("BEClient: try to connect " + this->config.battlEye.ip);
    BEClient bec     (this->config.battlEye.ip.c_str(), this->config.battlEye.port);
    bec.sendLogin    (this->config.battlEye.password.c_str());
    bec.readResponse (BE_LOGIN);
    if (bec.isLoggedIn()) {
        this->logger->log("BEClient: logged in!");
        bec.sendCommand  ("#shutdown");
        bec.readResponse (BE_COMMAND);
    } else {
        this->logger->log("BEClient: login failed!");
    }	
    bec.disconnect   ();
}

void Epochlib::beLock() {
    this->logger->log("BEClient: try to connect " + this->config.battlEye.ip);
    BEClient bec(this->config.battlEye.ip.c_str(), this->config.battlEye.port);
    bec.sendLogin(this->config.battlEye.password.c_str());
    bec.readResponse(BE_LOGIN);
    if (bec.isLoggedIn()) {
        this->logger->log("BEClient: logged in!");
        bec.sendCommand("#lock");
        bec.readResponse(BE_COMMAND);
    } else {
        this->logger->log("BEClient: login failed!");
    }
    bec.disconnect();
}

void Epochlib::beUnlock() {
    this->logger->log("BEClient: try to connect " + this->config.battlEye.ip);
    BEClient bec(this->config.battlEye.ip.c_str(), this->config.battlEye.port);
    bec.sendLogin(this->config.battlEye.password.c_str());
    bec.readResponse(BE_LOGIN);
    if (bec.isLoggedIn()) {
        this->logger->log("BEClient: logged in!");
        bec.sendCommand("#unlock");
        bec.readResponse(BE_COMMAND);
    } else {
        this->logger->log("BEClient: login failed!");
    }
    bec.disconnect();
}

