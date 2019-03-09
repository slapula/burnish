# burnish

`burnish` is a tool to manage deployments of EC2-powered applications.

## Overview

`burnish` is a high-level wrapper of the AWS API. By using `burnish`, teams can quickly deploy new code releases, view the status of their environments, and modify the state of running applications.

`burnish` makes certain assumptions about how applications are deployed:

- Each release is packaged as a single AWS EC2 AMI
- An application can be deployed to an auto scaling group and registered with an Elastic Load Balancer
- The application reports health and version information on well-known paths
- The application uses monotonically-incrementating integer release identifiers, e.g. r41, r42, r43…
- Two versions of the application can run simultaneously in separate auto scaling groups

In addition to managing deployments, `burnish` exposes tools to manage creating launch configurations, auto scaling groups, rotating instances in an auto scaling group, measuring ELB statistics, and marking deployment activities in New Relic.

At this time, `burnish` does not involve itself in:

- database migrations
- external notifications (e.g. pinging a Slack channel upon deploy completion)
- multiple-region application deployments
- non-EC2 AMI-based applications

This may change in future releases.

## Download

You can download a prebuilt binary [here](https://github.com/slapula/burnish/releases).

### Configure your AWS resources

`burnish` relies on a "universe" file that defines the environments and applications that it can deploy. [`universe.json.example`](util/universe.json.example) is a barebones example of a universe file. You **must** have a complete, functional universe file for `burnish` to function.

`burnish` does not impose any restrictions on your overall AWS architecture; it only assumes that your application is deployed as a single AMI to an auto scaling group, and is registered with an Elastic Load Balancer. You are free to configure your VPC and related network topology as you see fit.

`burnish` provides a [sample Terraform configuration](util/burnish.tf) that defines auto scaling groups, Cloud Watch alarms, and scaling policies. This configuration may be used as the basis for your own configuration, tailored to your application's specific configuration. See the [README](util/READMEmd) for instructions on how to use it and the expected output.

## Usage

### ELB Monitoring

Use the `burnish elb stats` to fetch real-time ELB metrics from CloudWatch.

```
$ deployer elb stats --app myapp --env prod
ELB: myapp-prod-lb
URL: https://console.aws.amazon.com/ec2/v2/home?region=us-east-1#LoadBalancers:search=myapp-prod-lb

interval	request     count	rps	500 errors	500 rate %	connection errors	avg. latency (msec)
1m0s		727		12.12	0		0.000%		0			38.047
5m0s		4084		13.61	0		0.000%		0			36.285
15m0s		12727		14.14	1		0.008%		0			36.504
1h0m0s		53524		14.87	5		0.009%		0			37.201
```

Use `burnish elb status` to retrieve information about instances registered with a load balancer. Omit `--app` to list all applications for an environment.

```
$ deployer elb status --app myapp --env prod
My App
ELB Name: myapp-prod-lb
ELB URL: https://console.aws.amazon.com/ec2/v2/home?region=us-east-1#LoadBalancers:search=myapp-prod-lb

ID			State		Name				Version		IP		ASG				Uptime
i-078e537840be51a20	InService	myapp-prod		r150		10.245.109.44	myapp-prod-green	119h19m6.355255633s
i-0b87d6e4c8a2cefdf	InService	myapp-prod		r150		10.245.109.101	myapp-prod-green	119h17m4.35530605s
```

### Application Deployment

Let's say you want to deploy a new release to production.

1. Create an AMI. Use packer, Ansible, or another tool to create an AMI with your new application release.  `burnish` is indifferent to how you make an AMI.
2. Tag your application AMI with `app` and `version` tags. The `app` tag should match the `app` value in your universe file, and the version should be a meaningful, unique string.
3. Perform a blue/green deploy of the new code. `burnish` will use the CLI flags to find the AMI, and then infer environment data (autoscaling groups, load balancers, etc.) from the universe file.
```
burnish deployment do --app application_name --env dev --version 42
```

### General usage

Use `burnish help` to see a complete set of command line operations.

```
USAGE:
    burnish [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -l, --log-level <STRING>    Logging Level: 'debug' for verbose, 'info' for terse (default: 'info')
    -p, --profile <STRING>      AWS Profile (Default: 'default')
    -r, --region <STRING>       AWS Region (Default: 'us-east-1')
    -u, --universe <FILE>       YAML Universe file

SUBCOMMANDS:
    autoscalegroup    create & manipulate autoscale groups
    debug             debug application inventory json
    deployment        perform deployment actions
    help              Prints this message or the help of the given subcommand(s)
    launchconfig      create a new launch config
    loadbalancer      create & manipulate elastic load balancers
    oneoff            launch or terminate a one-off instance
```