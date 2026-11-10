#ifndef YO3_METADATA_H
#define YO3_METADATA_H
#include "build/version.h"
#pragma pack(push, 1)
typedef struct { char magic[8]; unsigned abi; char release[65]; char commit[41]; } Yo3ModMetadata;
#pragma pack(pop)
#endif
