# Using `burnish.tf`

The included `burnish.tf` can be used to provision blue-green auto scaling groups that `burnish` expects in order to perform deployments/instance rotation. It assumes that a VPC, relevant subnets, and ELB(s) already exist for the application you wish to deploy.

- Create or update your `universe.json` file with sections for the new `env-name`. See: [`universe.json.example`](universe.json.example).
- Create a new launch configuration using the most recent release. For example:

```
UNIVERSE=s3://bucket-name/universe.json
APP=application-name
SERVICE=service-name
VERSION_NUM=86
ENV_NAME=env-name

burnish -u $UNIVERSE lc \
  --app $APP \
  --env $ENV_NAME \
  --version $VERSION_NUM \
  --instance-type m3.medium
```

- Run `terraform plan` using [`burnish.tf`](burnish.tf). Use a `terraform.tfvars` file based on:

```
app = "application-name"
service = "service-name"
elbs = ["application-name-env-name-lb"]
subnets = ["subnet-xxxxxxxx", "subnet-yyyyyyyy", "subnet-zzzzzzzz"]
env = "env-name"
lc = "LAUNCH_CONFIG_NAME_FROM_PREV_CMD"
az = ["us-east-1a", "us-east-1c", "us-east-1d"]
```

Complete the tfvars file with appropriate values. The value for `lc` should be the name of the launch configuration created in the previous step.

- Review the terraform plan. If it looks correct, run `terraform apply`.
- After terraform finishes applying the plan, a new instance (or instances) should come up in the green auto scaling group.