// SPDX-License-Identifier: LGPL-3.0-or-later
// Explicit isolated PAM check: never supplies a password or opens a session.
#include <security/pam_appl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#ifndef GXFP_PAM_SERVICE
#define GXFP_PAM_SERVICE "gxfp51b7-test"
#endif

static int conversation(int n,const struct pam_message **m,struct pam_response **r,void *ctx) {
 (void)ctx;
 struct pam_response *reply=calloc((size_t)n,sizeof(*reply));
 if(!reply)return PAM_BUF_ERR;
 for(int i=0;i<n;i++) {
  if(m[i]->msg_style==PAM_TEXT_INFO || m[i]->msg_style==PAM_ERROR_MSG)
   fprintf(stderr,"%s\n",m[i]->msg);
  else {free(reply);return PAM_CONV_ERR;}
 }
 *r=reply;return PAM_SUCCESS;
}
int main(int argc,char **argv) {
 if(argc!=2 || geteuid()!=0)return 2;
 pam_handle_t *pamh=NULL;
 struct pam_conv c={conversation,NULL};
 int result=pam_start(GXFP_PAM_SERVICE,argv[1],&c,&pamh);
 if(result==PAM_SUCCESS)result=pam_authenticate(pamh,0);
 if(result==PAM_SUCCESS)result=pam_acct_mgmt(pamh,0);
 if(pamh)pam_end(pamh,result);
 printf("PAM_RESULT=%d\n",result);
 return result==PAM_SUCCESS?0:1;
}
