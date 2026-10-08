// SPDX-License-Identifier: GPL-2.0-only OR LGPL-3.0-or-later
// Read-only original ACPI DSM 1; root-only, no firmware/key writes.
#include <linux/module.h>
#include <linux/acpi.h>
#include <linux/miscdevice.h>
#include <linux/fs.h>
#include <linux/dmi.h>
#include <linux/unaligned.h>
static const guid_t guid=GUID_INIT(0xcc58b68a,0x4479,0x4893,0xa8,0xbb,0x96,0x12,0x09,0xdb,0x59,0xe5);
static acpi_handle handle;
static ssize_t bios_read(struct file *f,char __user *buf,size_t count,loff_t *pos) {
 union acpi_object *o;ssize_t ret;u32 len;
 if(*pos>=885)return 0;
 o=acpi_evaluate_dsm(handle,&guid,0,1,NULL);
 if(!o)return -EIO;
 ret=-EINVAL;
 if(o->type!=ACPI_TYPE_BUFFER||o->buffer.length!=2048)goto out;
 len=get_unaligned_be32(o->buffer.pointer);
 if(len!=885||get_unaligned_le16(o->buffer.pointer+4)!=4||get_unaligned_le16(o->buffer.pointer+6)!=2)goto out;
 ret=simple_read_from_buffer(buf,count,pos,o->buffer.pointer+4,len);
 out:ACPI_FREE(o);return ret;
}
static const struct file_operations ops={.owner=THIS_MODULE,.read=bios_read};
static struct miscdevice dev={.minor=MISC_DYNAMIC_MINOR,.name="goodix_bios_sealed",.fops=&ops,.mode=0600};
static int __init start(void) {
 if(!dmi_match(DMI_SYS_VENDOR,"HUAWEI")||!dmi_match(DMI_PRODUCT_NAME,"MACHC-WAX9"))return -ENODEV;
 if(ACPI_FAILURE(acpi_get_handle(NULL,"\\_SB.SPBA",&handle)))return -ENODEV;
 return misc_register(&dev);
}
static void __exit stop(void){misc_deregister(&dev);}
module_init(start);module_exit(stop);MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Root-only read-only BIOS sealed-container reader for GXFP51B7");
