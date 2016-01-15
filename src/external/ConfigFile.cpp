#include "ConfigFile.hpp"
#include <fstream>

std::string trim(std::string const& source, char const* delims = " \t\r\n") {
	std::string result(source);
	std::string::size_type index = result.find_last_not_of(delims);
	if (index != std::string::npos)
		result.erase(++index);

	index = result.find_first_not_of(delims);
	if (index != std::string::npos)
		result.erase(0, index);
	else
		result.erase();
	return result;
}

// http://stackoverflow.com/questions/6089231/getting-std-ifstream-to-handle-lf-cr-and-crlf
std::istream& safeGetline(std::istream& is, std::string& t) {
	t.clear();

	// The characters in the stream are read one-by-one using a std::streambuf.
	// That is faster than reading them one-by-one using the std::istream.
	// Code that uses streambuf this way must be guarded by a sentry object.
	// The sentry object performs various tasks,
	// such as thread synchronization and updating the stream state.

	std::istream::sentry se(is, true);
	std::streambuf* sb = is.rdbuf();

	for (;;) {
		int c = sb->sbumpc();
		switch (c) {
		case '\n':
			return is;
		case '\r':
			if (sb->sgetc() == '\n')
				sb->sbumpc();
			return is;
		case EOF:
			// Also handle the case when the last line has no line ending
			if (t.empty())
				is.setstate(std::ios::eofbit);
			return is;
		default:
			t += (char)c;
		}
	}
}

ConfigFile::ConfigFile(std::string const& configFile) {
	std::ifstream file(configFile.c_str());

	std::string line;
	std::string name;
	std::string value;
	std::string inSection;
	int posEqual;
	while (safeGetline(file, line)) {

		if (!line.length()) continue;

		if (line[0] == '#') continue;
		if (line[0] == ';') continue;

		if (line[0] == '[') {
			inSection = trim(line.substr(1, line.find(']') - 1));
			continue;
		}

		posEqual = line.find('=');
		name = trim(line.substr(0, posEqual));
		value = trim(line.substr(posEqual + 1));

		content_[inSection + '/' + name] = Chameleon(value);
	}
}

Chameleon const& ConfigFile::Value(std::string const& section, std::string const& entry) const {

	std::map<std::string, Chameleon>::const_iterator ci = content_.find(section + '/' + entry);

	if (ci == content_.end()) throw "does not exist";

	return ci->second;
}

Chameleon const& ConfigFile::Value(std::string const& section, std::string const& entry, double value) {
	try {
		return Value(section, entry);
	}
	catch (const char *) {
		return content_.insert(std::make_pair(section + '/' + entry, Chameleon(value))).first->second;
	}
}

Chameleon const& ConfigFile::Value(std::string const& section, std::string const& entry, std::string const& value) {
	try {
		return Value(section, entry);
	}
	catch (const char *) {
		return content_.insert(std::make_pair(section + '/' + entry, Chameleon(value))).first->second;
	}
}