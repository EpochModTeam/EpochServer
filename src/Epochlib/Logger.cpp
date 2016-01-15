#include "Logger.hpp"
#include <fstream>
#include <ctime>
#include <iomanip>

Logger::Logger(std::string _logFile) {
	this->logFile = _logFile;
}
void Logger::log(std::string _message) {
	std::ofstream logFile;

	logFile.open(this->logFile, std::ios::app);
	if (logFile.good()) {
#ifdef __linux__
                char outstr[200];
                time_t t       = time (NULL);
                struct tm *tmp = localtime (&t);
                if ( tmp != NULL && strftime(outstr, sizeof(outstr), "%Y-%m-%d %H:%M:%S ", tmp) != 0 ) {
                    logFile << outstr << _message << std::endl;
		}
#else
		std::time_t t = std::time(nullptr);
		std::tm tm = *std::localtime(&t);
		logFile << std::put_time(&tm, "%Y-%m-%d %H:%M:%S ") << _message << std::endl;
#endif
		logFile.close();
	}
}