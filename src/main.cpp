#include "Epochlib/Epochlib.hpp"
#ifdef WIN32
	#include <windows.h>
	EXTERN_C IMAGE_DOS_HEADER __ImageBase;
#else
	#include <fcntl.h>
	#include <limits.h>
	#include <unistd.h>
	#include <string.h>
#endif
#include <sstream>
#include <fstream>
#include <vector>
#include <thread>

#define SEPARATOR "|"

#ifdef WIN32
extern "C" {
	__declspec (dllexport) void __stdcall RVExtension(char *output, int outputSize, const char *function);
}
#else
extern "C" {
        void RVExtension (char* output, int outputSize, const char* function);
}
#endif

Epochlib *EpochLibrary;

std::vector<std::string> &split(const std::string &s, char delim, std::vector<std::string> &elems) {
	std::stringstream ss(s);
	std::string item;

	while (std::getline(ss, item, delim)) {
		elems.push_back(item);
	}

	return elems;
}
std::vector<std::string> split(const std::string &s, char delim) {
	std::vector<std::string> elems;
	split(s, delim, elems);
	return elems;
}

std::string getProfileFolder() {
	std::string profileFolder = "";
	int numCmdLineArgs = 0;
	std::vector<std::string> commandLine;

#ifdef WIN32
	LPCWSTR cmdLine = GetCommandLineW();
	LPWSTR *cmdLineArgs = CommandLineToArgvW(cmdLine, &numCmdLineArgs);

	commandLine.reserve(numCmdLineArgs);

	for (int i = 0; i < numCmdLineArgs; i++) {
		std::wstring args(cmdLineArgs[i]);
		std::string utf8(args.begin(), args.end());
		commandLine.push_back(utf8);
	}
#else
	std::stringstream cmdlinePath;
	cmdlinePath << "/proc/" << (int)getpid() << "/cmdline";
	int cmdlineFile = open(cmdlinePath.str().c_str(), O_RDONLY);

	char cmdLineArgs[PATH_MAX];
	if ((numCmdLineArgs = read(cmdlineFile, cmdLineArgs, PATH_MAX)) > 0) {
		std::string procCmdline;
		procCmdline.assign(cmdLineArgs, numCmdLineArgs - 1);
		commandLine = split(procCmdline, '\0');
	}
#endif

	for (std::vector<std::string>::iterator it = commandLine.begin(); it != commandLine.end(); it++) {
		std::string starter = "-profiles=";
		if (it->length() < starter.length()) {
			continue;
		}

		std::string compareMe = it->substr(0, starter.length());
		if (compareMe.compare(starter) != 0) {
			continue;
		}

		profileFolder = it->substr(compareMe.length());
	}

	return profileFolder;
}

std::string join(std::vector<std::string> split, int index) {
	std::stringstream joinedString;

	for (std::vector<std::string>::iterator
		it = split.begin() + index;
		it != split.end();
	) {
		joinedString << ((split.begin() + index) != it ? SEPARATOR : "") << *it;
		it++;
	}

	return joinedString.str();
}

/*
	Handler
*/
std::string handler000(std::vector<std::string> _param) {
	return EpochLibrary->getConfig();
}
std::string handler001(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		std::thread process(std::bind(&Epochlib::initPlayerCheck, EpochLibrary, atoll(_param[0].c_str())));
		process.detach();
	}

	return "";
}
std::string handler100(std::vector<std::string> _param) {
	if (_param.size() >= 3) {
		return EpochLibrary->setTemp(_param[0], _param[1], join(_param, 2));
	}

	return "";
}
std::string handler101(std::vector<std::string> _param) {
	if (_param.size() >= 3) {
		std::thread process(std::bind(&Epochlib::setTemp, EpochLibrary, _param[0], _param[1], join(_param, 2)));
		process.detach();
	}

	return "";
}

std::string handler110(std::vector<std::string> _param) {
	if (_param.size() >= 3) {
		return EpochLibrary->set(_param[0], _param[1], join(_param, 2));
	}

	return "";
}
std::string handler111(std::vector<std::string> _param) {
	if (_param.size() >= 3) {
		std::thread process(std::bind(&Epochlib::set, EpochLibrary, _param[0], _param[1], join(_param, 2)));
		process.detach();
	}

	return "";
}

std::string handler120(std::vector<std::string> _param) {
	if (_param.size() >= 4) {
		return EpochLibrary->setex(_param[0], _param[1], _param[2], join(_param,3));
	}

	return "";
}
std::string handler121(std::vector<std::string> _param) {
	if (_param.size() >= 4) {
		std::thread process(std::bind(&Epochlib::setex, EpochLibrary, _param[0], _param[1], _param[2], join(_param, 3)));
		process.detach();
	}

	return "";
}

std::string handler130(std::vector<std::string> _param) {
	if (_param.size() == 2) {
		return EpochLibrary->expire(_param[0], _param[1]);
	}

	return "";
}
std::string handler131(std::vector<std::string> _param) {
	if (_param.size() == 2) {
		std::thread process(std::bind(&Epochlib::expire, EpochLibrary, _param[0], _param[1]));
		process.detach();
	}

	return "";
}

std::string handler141(std::vector<std::string> _param) {
	if (_param.size() >= 2) {
		std::thread process(std::bind(&Epochlib::setbit, EpochLibrary, _param[0], _param[1], _param[2]));
		process.detach();
	}

	return "";
}

std::string handler200(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		return EpochLibrary->get(join(_param, 0));
	}

	return "";
}
std::string handler210(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		return EpochLibrary->getTtl(join(_param, 0));
	}

	return "";
}

// Get Range
std::string handler220(std::vector<std::string> _param) {
	if (_param.size() >= 3) {
		return EpochLibrary->getRange(_param[0], _param[1], _param[2]);
	}

	return "";
}



std::string handler240(std::vector<std::string> _param) {
	if (_param.size() == 2) {
		return EpochLibrary->getbit(_param[0], _param[1]);
	}

	return "";
}

std::string handler250(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		return EpochLibrary->exists(join(_param, 0));
	}

	return "";
}

std::string handler300(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		return EpochLibrary->ttl(join(_param, 0));
	}

	return "";
}

std::string handler400(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		return EpochLibrary->del(join(_param, 0));
	}

	return "";
}

std::string handler500(std::vector<std::string> _param) {
	return EpochLibrary->ping();
}

std::string handler510(std::vector<std::string> _param) {
	return EpochLibrary->getCurrentTime();
}

std::string handler600(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		return EpochLibrary->lpopWithPrefix("CMD:", join(_param, 0));
	}

	return "";
}

std::string handler700(std::vector<std::string> _param) {
	if (_param.size() >= 2) {
		return EpochLibrary->log(_param[0], join(_param, 1));
	}

	return "";
}
std::string handler701(std::vector<std::string> _param) {
	if (_param.size() >= 2) {
		std::thread process(std::bind(&Epochlib::log, EpochLibrary, _param[0], join(_param, 1)));
		process.detach();
	}

	return "";
}

std::string handler800(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		return EpochLibrary->updatePublicVariable(_param);
	}

	return "";
}
std::string handler801(std::vector<std::string> _param) {
	if (_param.size() >= 1) {
		std::thread process(std::bind(&Epochlib::updatePublicVariable, EpochLibrary, _param));
		process.detach();
	}

	return "";
}

std::string handler810(std::vector<std::string> _param) {
	if (_param.size() == 1) {
		return EpochLibrary->getRandomString(atoi(_param[0].c_str()));
	}
	else if(_param.size() == 0) {
		return EpochLibrary->getRandomString(1);
	}

	return "";
}

std::string handler820(std::vector<std::string> _param) {
	if (_param.size() >= 2) {
		return EpochLibrary->addBan(atoll(_param[0].c_str()), join(_param, 1));
	}

	return "";
}
std::string handler821(std::vector<std::string> _param) {
	if (_param.size() >= 2) {
		std::thread process(std::bind(&Epochlib::addBan, EpochLibrary, atoll(_param[0].c_str()), join(_param, 1)));
		process.detach();
	}

	return "";
}

std::string handler830(std::vector<std::string> _param) {
	return EpochLibrary->increaseBancount();
}

// Battleye Integration

// say  (Message)
std::string handler901(std::vector<std::string> _param) {
        if (_param.size() > 0) {
        	std::thread process(std::bind(&Epochlib::beBroadcastMessage, EpochLibrary, _param[0]));
		process.detach();
	}

        return "";
}

// kick (playerUID, Message)
std::string handler911(std::vector<std::string> _param) {
        if (_param.size() > 1) {
        	std::thread process(std::bind(&Epochlib::beKick, EpochLibrary, _param[0], _param[1]));
		process.detach();
	}

	return "";
}

// ban  (playerUID, Message, Duration)
std::string handler921(std::vector<std::string> _param) {
        if (_param.size() > 2) {
        	std::thread process(std::bind(&Epochlib::beBan, EpochLibrary, _param[0], _param[1], _param[2]));
        	process.detach();
	}

	return "";
}

// lock
std::string handler931(std::vector<std::string> _param) {
	std::thread process(std::bind(&Epochlib::beLock, EpochLibrary));
	process.detach();

	return "";
}

// unlock
std::string handler930(std::vector<std::string> _param) {
	std::thread process(std::bind(&Epochlib::beUnlock, EpochLibrary));
	process.detach();

	return "";
}

// shutdown
std::string handler991(std::vector<std::string> _param) {
	std::thread process(std::bind(&Epochlib::beShutdown, EpochLibrary));
	process.detach();

	return "";
}


#ifdef EPOCHLIB_TEST
std::string handlerT100(std::vector<std::string> _param) {
	return EpochLibrary->getServerMD5();
}
#endif

/*
	RVExtension (Extension main call)
*/
#ifdef WIN32
void __stdcall RVExtension(char *_output, int _outputSize, const char *_function) {
#elif __linux__
void RVExtension(char *_output, int _outputSize, const char *_function) {
#endif
	std::vector<std::string> rawCmd = split(std::string(_function), SEPARATOR[0]);
	std::string hiveOutput = "";

	if (EpochLibrary == NULL) {
		std::string configPath;

#ifdef WIN32
		// Get file path
		char DllPath[MAX_PATH];
		GetModuleFileName((HINSTANCE)&__ImageBase, DllPath, _countof(DllPath));
		std::string filePath(DllPath);

		// Get file folder and use it as config folder
		configPath = filePath.substr(0, filePath.find_last_of("\\/"));
#elif __linux__
		configPath = "@epochhive";
#endif

		EpochLibrary = new Epochlib(configPath, getProfileFolder(), _outputSize);
	}

	if (rawCmd.size() > 0) {
		// Get config
		if (rawCmd[0] == "000") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler000(rawCmd);
		}
		// Initial player check
		else if (rawCmd[0] == "001") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler001(rawCmd);
		}
		// SET temp
		else if (rawCmd[0] == "100") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler100(rawCmd);
		}
		else if (rawCmd[0] == "101") { // Async
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler101(rawCmd);
		}
		// SET
		else if (rawCmd[0] == "110") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler110(rawCmd);
		}
		else if (rawCmd[0] == "111") { // Async
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler111(rawCmd);
		}
		// SETEX
		else if (rawCmd[0] == "120") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler120(rawCmd);
		}
		else if (rawCmd[0] == "121") { // Async
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler121(rawCmd);
		}
		// EXPIRE
		else if (rawCmd[0] == "130") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler130(rawCmd);
		}
		else if (rawCmd[0] == "131") { // Async
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler131(rawCmd);
		}
		// SETBIT
		else if (rawCmd[0] == "141") { // Async
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler141(rawCmd);
		}
		// GET
		else if (rawCmd[0] == "200") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler200(rawCmd);
		}
		else if (rawCmd[0] == "210") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler210(rawCmd);
		}
		else if (rawCmd[0] == "220") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler220(rawCmd);
		}
		else if (rawCmd[0] == "240") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler240(rawCmd);
		}
		// TTL
		else if (rawCmd[0] == "300") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler300(rawCmd);
		}
		else if (rawCmd[0] == "400") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler400(rawCmd);
		}
		// Utilities
		else if (rawCmd[0] == "500") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler500(rawCmd);
		}
		else if (rawCmd[0] == "510") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler510(rawCmd);
		}
		// Array
		else if (rawCmd[0] == "600") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler600(rawCmd);
		}
		// Logging
		else if (rawCmd[0] == "700") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler700(rawCmd);
		}
		else if (rawCmd[0] == "701") { // Async
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler701(rawCmd);
		}
		// Antihack
		else if (rawCmd[0] == "800") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler800(rawCmd);
		}
		else if (rawCmd[0] == "801") { // Async
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler801(rawCmd);
		}
		else if (rawCmd[0] == "810") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler810(rawCmd);
		}
		else if (rawCmd[0] == "820") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler820(rawCmd);
		}
		else if (rawCmd[0] == "821") { // Async
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler821(rawCmd);
		}
		else if (rawCmd[0] == "830") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler830(rawCmd);
		}
		// Battleye Integration
		else if (rawCmd[0] == "901") { // say  (Message)
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler901(rawCmd);
		}
		else if (rawCmd[0] == "911") { // kick (playerUID, Message)
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler911(rawCmd);
		}
		else if (rawCmd[0] == "921") { // ban  (playerUID, Message, Duration)
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler921(rawCmd);
		}
		else if (rawCmd[0] == "930") { // Unlock
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler930(rawCmd);
		}
		else if (rawCmd[0] == "931") { // Lock Server
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler931(rawCmd);
		}
		else if (rawCmd[0] == "991") { // shutdown
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handler991(rawCmd);
		}
#ifdef EPOCHLIB_TEST
		else if (rawCmd[0] == "T100") {
			rawCmd.erase(rawCmd.begin(), rawCmd.begin() + 1);
			hiveOutput = handlerT100(rawCmd);
		}
#endif
		else {
			hiveOutput = "Unkown command " + rawCmd[0];
			//std::string OStext = std::to_string(_outputSize);
			//hiveOutput = "Unkown command " + rawCmd[0] + " Max Output " + OStext;
		}
	}
	else {
		hiveOutput = "0.5.1.8";
	}

	strncpy(_output, hiveOutput.c_str(), _outputSize);

	#ifdef __linux__
		_output[_outputSize - 1] = '\0';
		return;
	#endif
}
