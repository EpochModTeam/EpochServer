#include <vector>
#include <string>
#include <sstream>

#ifndef SQF_H
#define SQF_H

class SQF {
private:
	std::vector<std::string> arrayStack;

public:
	SQF();
	~SQF();
	void push_str(const char *String);
	void push_str(const char *String, int Flag);
	void push_number(long long int Number);
	void push_number(const char *Number, size_t NumberSize);
	void push_array(const char *String);
	void push_array(std::string String);
	std::string toArray();
};

#endif
