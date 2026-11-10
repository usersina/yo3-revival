#ifndef YO3_METADATA_H
#define YO3_METADATA_H
#include "build/version.h"
#pragma pack(push, 1)
typedef struct { char magic[8]; unsigned abi; char release[65]; char commit[41]; } Yo3ModMetadata;
#pragma pack(pop)
enum { YO3_API_STRING = 1, YO3_API_INT, YO3_API_URL, YO3_API_CONNECT };
#endif
