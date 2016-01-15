#ifdef WIN32
    #include <winsock2.h>
    #include <ws2tcpip.h>
    #pragma comment(lib,"ws2_32.lib") //Winsock Library
    #include <stdint.h>
#elif __linux__
    #include <stdio.h>
    #include <errno.h>
    #include <string.h>
    #include <sys/types.h>
    #include <sys/socket.h>
    #include <netinet/in.h>
    #include <arpa/inet.h>
    #include <unistd.h>
    #include <sys/time.h>
    
    #define closesocket    close
    #define INVALID_SOCKET -1
    #define SOCKET_ERROR   -1
#endif

#include <map>
#include <string>

#ifndef __BECLIENT_H__
#define __BECLIENT_H__

#define BE_LOGIN   0x00
#define BE_COMMAND 0x01
#define BE_MESSAGE 0x02

class BEClient {
private:
    const char*        ip;
    uint16_t           port;
    struct sockaddr_in saddr;
    int                sock;
    unsigned char      sequence;
    bool               loggedIn;
    std::string        result;
    std::map<int, std::string> part_result;

public:
    BEClient (const char* _ip, uint16_t _port);
   ~BEClient ();

    bool     isLoggedIn       ();
    uint32_t getCRC32         (unsigned char *_addr, int _size);
    bool     sendPacket       (unsigned char cmd, const char* data);
    bool     sendLogin        (const char* passwd);
    bool     sendCommand      (const char* cmd);
    void     readResponse     (unsigned char cmd);
    bool     readPacket       (unsigned char cmd);
    int      getPlayerSlot    (std::string players);
    void     disconnect       ();
    void     hexDump          (void *addr, int len);
};

#endif
