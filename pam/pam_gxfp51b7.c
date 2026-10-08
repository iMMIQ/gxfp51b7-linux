// SPDX-License-Identifier: LGPL-3.0-or-later
// Optional fingerprint authentication; every error falls back to the PAM stack.
// Root-owned runtime must be independently validated before this is installed.
#define _GNU_SOURCE
#include <security/pam_modules.h>
#include <security/pam_ext.h>
#include <sys/resource.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <fcntl.h>
#include <signal.h>
#include <time.h>
#include <errno.h>
#include <string.h>

PAM_EXTERN int pam_sm_authenticate(pam_handle_t *pamh, int flags,
                                  int argc, const char **argv) {
 const char *user = NULL;
 if(geteuid()!=0 || pam_get_user(pamh,&user,NULL)!=PAM_SUCCESS || !user || !*user)
  return PAM_AUTHINFO_UNAVAIL;
 const char *allowed=NULL;
 for(int i=0;i<argc;i++)if(strncmp(argv[i],"user=",5)==0)allowed=argv[i]+5;
 if(!allowed || strcmp(user,allowed)!=0)return PAM_AUTHINFO_UNAVAIL;
 if(!(flags&PAM_SILENT))pam_info(pamh,"Touch the enrolled index finger (up to 15 seconds)");
 pid_t pid=fork();
 if(pid<0)return PAM_AUTHINFO_UNAVAIL;
 if(pid==0) {
  struct rlimit limit={0,0};setrlimit(RLIMIT_CORE,&limit);
  setpgid(0,0);
  int fd=open("/dev/null",O_RDWR);
  if(fd<0)_exit(2);
  dup2(fd,STDIN_FILENO);dup2(fd,STDOUT_FILENO);dup2(fd,STDERR_FILENO);
  if(fd>2)close(fd);
  close_range(3,~0U,0);
  char *args[]={"/usr/bin/python","-I","/usr/local/lib/gxfp51b7/verify.py",(char*)user,NULL};
  char *env[]={"PATH=/usr/bin:/bin","LANG=C.UTF-8",NULL};
  execve(args[0],args,env);_exit(2);
 }
 struct timespec started,now,pause={0,50000000};clock_gettime(CLOCK_MONOTONIC,&started);
 int status=0;
 for(;;) {
  pid_t r=waitpid(pid,&status,WNOHANG);
  if(r==pid)break;
  if(r<0 && errno!=EINTR)return PAM_AUTHINFO_UNAVAIL;
  clock_gettime(CLOCK_MONOTONIC,&now);
  if(now.tv_sec-started.tv_sec>=15) {
   kill(-pid,SIGTERM);kill(pid,SIGTERM);
   nanosleep(&pause,NULL);kill(-pid,SIGKILL);kill(pid,SIGKILL);
   while(waitpid(pid,&status,0)<0 && errno==EINTR) {}
   return PAM_AUTHINFO_UNAVAIL;
  }
  nanosleep(&pause,NULL);
 }
 return WIFEXITED(status)&&WEXITSTATUS(status)==0 ? PAM_SUCCESS : PAM_AUTHINFO_UNAVAIL;
}

PAM_EXTERN int pam_sm_setcred(pam_handle_t *pamh,int flags,int argc,const char **argv) {
 (void)pamh;(void)flags;(void)argc;(void)argv;return PAM_SUCCESS;
}
