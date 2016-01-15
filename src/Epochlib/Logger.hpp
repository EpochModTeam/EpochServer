#include <string>

#ifndef __LOGGER_H__
#define __LOGGER_H__

class Logger {
private:
	std::string logFile;

public:
	Logger(std::string LogFile);
	void log(std::string Message);
};

#endif