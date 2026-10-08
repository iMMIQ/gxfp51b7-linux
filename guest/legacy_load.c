// SPDX-License-Identifier: LGPL-3.0-or-later
// Userspace SGXS loading through Intel's unmodified legacy driver.
// Intended for the isolated Ubuntu guest, not the host kernel.
#define _GNU_SOURCE
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <signal.h>
#include <sys/resource.h>
#include <cpuid.h>
#include "sgx_user.h"

static uint64_t u64(const void *p) { uint64_t v; memcpy(&v,p,8);return v; }
static uint32_t u32(const void *p) { uint32_t v; memcpy(&v,p,4);return v; }
static void set64(void *p,uint64_t v) {memcpy(p,&v,8);}
static void set32(void *p,uint32_t v) {memcpy(p,&v,4);}
static void *aligned(size_t n) {void *p=NULL;if(posix_memalign(&p,4096,n))abort();memset(p,0,n);return p;}
struct enclave {int fd;void *base;uint64_t size,tcs;uint8_t *sig;};
static int error(const char *what,int result) {
 fprintf(stderr,"%s failed: result=%d errno=%d (%s)\n",what,result,errno,strerror(errno)); return -1;
}
static int load(const char *image,const char *signature,const uint8_t token[304],struct enclave *e) {
 uint8_t rec[64],*source=aligned(4096),*si=aligned(4096),*secs=aligned(4096);
 FILE *s=fopen(signature,"rb"),*f=fopen(image,"rb");e->sig=aligned(4096);
 if(!s||!f)return error("open file",-1);
 if(fread(e->sig,1,1808,s)!=1808)return -1;
 fclose(s);
 if(fread(rec,1,64,f)!=64||memcmp(rec,"ECREATE",8))return -1;
 e->size=u64(rec+12); if(e->size<8192||e->size>0x8000000||(e->size&(e->size-1)))return -1;
 e->fd=open("/dev/isgx",O_RDWR);if(e->fd<0)return error("open isgx",-1);
 e->base=mmap(NULL,e->size,PROT_READ|PROT_WRITE|PROT_EXEC,MAP_SHARED,e->fd,0);
 if(e->base==MAP_FAILED)return error("mmap",-1);
 set64(secs,e->size);set64(secs+8,(uint64_t)e->base);set32(secs+16,u32(rec+8));
 set32(secs+20,u32(e->sig+900));memcpy(secs+48,e->sig+928,16);
 struct sgx_enclave_create create={(uint64_t)secs};int ret=ioctl(e->fd,SGX_IOC_ENCLAVE_CREATE,&create);
 if(ret)return error("ECREATE",ret);
 unsigned pages=0;
 while(fread(rec,1,64,f)==64) {
  if(memcmp(rec,"EADD\0\0\0",8))return error("bad SGXS page record",-1);
  uint64_t offset=u64(rec+8),flags=u64(rec+16);unsigned short chunks=0;
  if(offset>=e->size||(offset&4095))return -1;
  memset(source,0,4096);memset(si,0,64);set64(si,flags);
  for(;;) {
   long pos=ftell(f);if(fread(rec,1,64,f)!=64){fseek(f,pos,SEEK_SET);break;}
   if(memcmp(rec,"EEXTEND",8)){fseek(f,pos,SEEK_SET);break;}
   uint64_t ext=u64(rec+8);
   if(ext<offset||ext>=offset+4096||(ext&255))return -1;
   if(fread(source+ext-offset,1,256,f)!=256)return -1;
   chunks|=1U<<((ext-offset)/256);
  }
  struct sgx_enclave_add_page add={(uint64_t)e->base+offset,(uint64_t)source,(uint64_t)si,chunks};
  ret=ioctl(e->fd,SGX_IOC_ENCLAVE_ADD_PAGE,&add);if(ret)return error("EADD",ret);
  if(flags==0x100&&!e->tcs)e->tcs=(uint64_t)e->base+offset;
  pages++;
 }
 fclose(f);fprintf(stderr,"Loaded %u pages, size %lu bytes\n",pages,e->size);
 struct sgx_enclave_init init={(uint64_t)e->base,(uint64_t)e->sig,(uint64_t)token};
 ret=ioctl(e->fd,SGX_IOC_ENCLAVE_INIT,&init);if(ret)return error("EINIT",ret);
 fprintf(stderr,"EINIT succeeded\n");free(source);free(si);free(secs);return 0;
}
extern uint64_t enter_enclave(uint64_t tcs,int64_t call,void *ms);
static uint8_t sealed_input[4096];
static uint16_t sealed_len;
uint64_t handle_ocall(uint64_t index,void *ms) {
 if(index<=1) {uint8_t *p=ms;void *buffer=(void *)u64(p+16);uint64_t count=u64(p+24);
  if(!buffer||count>65536) {set32(ms,-1);return 0;}
  uint32_t hdr[3]={0x43505247,(uint32_t)index,(uint32_t)count};
  if(fwrite(hdr,1,12,stdout)!=12)return 1;
  if(index==1&&fwrite(buffer,1,count,stdout)!=count)return 1;
  fflush(stdout);int64_t result;
  if(fread(&result,1,8,stdin)!=8)return 1;
  if(index==0&&result>0) {if((uint64_t)result>count||fread(buffer,1,result,stdin)!=(size_t)result)return 1;}
  set32(ms,(uint32_t)result);return 0;
 }
 if(index==14) {const char *msg=(void *)u64((char *)ms+32);if(msg)fprintf(stderr,"Goodix log: %.240s\n",msg);return 0;}
 if(index==2||index==3) {set32(ms,0);return 0;}
 if(index==4||index==5||index==10||index==12||index==13)return 0;
 if(index==6) {uint8_t *p=ms;uint16_t cap;memcpy(&cap,p+16,2);void *out=(void *)u64(p+8),*outlen=(void *)u64(p+24);
  if(!out||!outlen||!sealed_len||cap<sealed_len) {set32(ms,-1);return 0;}
  memcpy(out,sealed_input,sealed_len);memcpy(outlen,&sealed_len,2);set32(ms,0);return 0;}
 if(index==9) {fprintf(stderr,"Handshake completion callback status=%u\n",u32(ms));uint32_t h[3]={0x43505247,4,u32(ms)};fwrite(h,1,12,stdout);fflush(stdout);return 0;}
 if(index==11&&!ms)return 0;
 if(index==8) {set32(ms,0);return 0;}
 if(index==7) {set32(ms,-1);return 0;}
 fprintf(stderr,"Unsupported ocall index=%lu ms=%p\n",index,ms);return 1;
}
static int initialize(struct enclave *e) {
 uint8_t features[256]={0};set64(features,0x267ff);set64(features+16,1ULL<<63);
 uint64_t status=enter_enclave(e->tcs,-1,features);
 fprintf(stderr,"Runtime initialization status=0x%lx\n",status);return status? -1:0;
}
int main(int argc,char **argv) {
 struct rlimit core={0,0};setrlimit(RLIMIT_CORE,&core);
 sealed_len=fread(sealed_input,1,885,stdin);mlock(sealed_input,sizeof(sealed_input));
 setvbuf(stdout,NULL,_IONBF,0);setvbuf(stderr,NULL,_IONBF,0);
 if(argc!=6){fprintf(stderr,"Usage: legacy_load LE.sgxs production.sigstruct whitelist target.sigstruct\n");return 2;}
 uint8_t token[304]={0};struct enclave e={0};
 if(load(argv[1],argv[2],token,&e)||initialize(&e))return 1;
 FILE *f=fopen(argv[3],"rb");if(!f)return 1;fseek(f,0,SEEK_END);long len=ftell(f);rewind(f);
 if(len<248||len>65536)return 1;
 uint8_t *cert=malloc(len);
 if(fread(cert,1,len,f)!=(size_t)len)return 1;
 fclose(f);
 struct {uint64_t retval;void *cert;uint32_t len;uint32_t pad;} reg={0,cert,len,0};
 uint64_t status=enter_enclave(e.tcs,1,&reg);
 fprintf(stderr,"Whitelist ecall status=0x%lx retval=0x%lx\n",status,reg.retval);
 if(status||reg.retval)return 1;
 uint8_t sig[1808],signer[32];f=fopen(argv[4],"rb");if(!f||fread(sig,1,sizeof(sig),f)!=sizeof(sig))return 1;fclose(f);
 // Signer is the already verified SHA256 of the target SIGSTRUCT RSA modulus.
 const uint8_t goodix_signer[32]={0x2f,0xe0,0x2f,0x62,0xd2,0xe4,0x4e,0x51,0x47,0x44,0x30,0x63,0x28,0x39,0xbd,0xb7,0xc9,0x05,0xbc,0xba,0xe1,0x48,0x3b,0x91,0x12,0x59,0xd4,0xf2,0x67,0x9d,0x22,0xc2};
 memcpy(signer,goodix_signer,32);
 struct {uint64_t retval;void *measurement,*signer,*attributes,*token;} request={0,sig+960,signer,sig+928,token};
 status=enter_enclave(e.tcs,0,&request);
 fprintf(stderr,"Token ecall status=0x%lx retval=0x%lx valid=%u\n",status,request.retval,u32(token));
 if(status||request.retval||u32(token)!=1)return 1;
 fprintf(stderr,"Goodix launch token generated successfully (memory only).\n");
 munmap(e.base,e.size);close(e.fd);free(e.sig);memset(&e,0,sizeof(e));
 if(load(argv[5],argv[4],token,&e)||initialize(&e))return 1;
 uint16_t image_len=10240,nav_len=3200;uint32_t sensor=12;
 struct {uint64_t retval;void *image_len,*nav_len,*sensor;uint32_t retry,max;} buffers={0,&image_len,&nav_len,&sensor,0,1};
 status=enter_enclave(e.tcs,2,&buffers);
 fprintf(stderr,"Image buffer initialization status=0x%lx retval=0x%lx\n",status,buffers.retval);
 if(status||buffers.retval)return 1;
 struct {uint32_t retval;uint32_t loglevel;} tls={0,7};
 status=enter_enclave(e.tcs,0x27,&tls);
 fprintf(stderr,"Goodix TLS initialization status=0x%lx retval=%d\n",status,(int)tls.retval);
 if(status||tls.retval)return 1;
 uint32_t ready[3]={0x43505247,2,0};fwrite(ready,1,12,stdout);fflush(stdout);
 int64_t ack;if(fread(&ack,1,8,stdin)!=8||ack)return 1;
 uint32_t handshake=0;status=enter_enclave(e.tcs,0x29,&handshake);
 fprintf(stderr,"TLS handshake ecall status=0x%lx retval=%d\n",status,(int)handshake);
 if(!status && (int32_t)handshake>=0) {
  uint8_t image[10560]={0};struct {void *image;uint16_t length;} get={image,sizeof(image)};
  uint64_t st=enter_enclave(e.tcs,0x15,&get);
  fprintf(stderr,"Source image export status=0x%lx\n",st);
  if(!st){uint32_t h[3]={0x43505247,5,sizeof(image)};fwrite(h,1,12,stdout);fwrite(image,1,sizeof(image),stdout);fflush(stdout);}
 }
 uint32_t done[3]={0x43505247,3,handshake};fwrite(done,1,12,stdout);fflush(stdout);
 munmap(e.base,e.size);close(e.fd);free(e.sig);return status?1:0;
}
