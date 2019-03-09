# This is a terraform script to create the ASG required for blue/green deploys

provider "aws" {
    region = "us-east-1"
}

variable "app" {
  description = "name of the application"
}

variable "service" {
  description = "name of the service that the app runs as"
}

variable "elbs" {
  description = "list of ELBs to associate with the asg"
  type = "list"
}

variable "subnets" {
  description = "list of subnets to associate with the asg"
  type = "list"
}

variable "env" {
  description = "name of the environment"
}

variable "lc" {
  description = "existing launch config to use"
}

variable "az" {
  description = "list of availability zones to launch instances in"
  type = "list"
  default = ["us-east-1a", "us-east-1c"]
}

resource "aws_autoscaling_group" "asg-blue" {
  availability_zones        = ["${var.az}"]
  name                      = "${var.app}-${var.env}-blue"
  max_size                  = 0
  min_size                  = 0
  desired_capacity          = 0
  health_check_grace_period = 300
  health_check_type         = "ELB"
  launch_configuration      = "${var.lc}"
  load_balancers            = ["${var.elbs}"]
  vpc_zone_identifier       = ["${var.subnets}"]
  enabled_metrics           = ["GroupMinSize", "GroupMaxSize", "GroupDesiredCapacity", "GroupInServiceInstances", "GroupPendingInstances", "GroupStandbyInstances", "GroupTerminatingInstances", "GroupTotalInstances"]

  tag {
    key                 = "Name"
    value               = "${var.app}-${var.env}"
    propagate_at_launch = true
  }

  tag {
    key                 = "app"
    value               = "${var.app}"
    propagate_at_launch = true
  }

  tag {
    key                 = "service"
    value               = "${var.service}"
    propagate_at_launch = true
  }

  tag {
    key                 = "env"
    value               = "${var.env}"
    propagate_at_launch = true
  }

}

resource "aws_autoscaling_group" "asg-green" {
  availability_zones        = ["${var.az}"]
  name                      = "${var.app}-${var.env}-green"
  max_size                  = 12
  min_size                  = 1
  desired_capacity          = 1
  health_check_grace_period = 300
  health_check_type         = "ELB"
  launch_configuration      = "${var.lc}"
  load_balancers            = ["${var.elbs}"]
  vpc_zone_identifier       = ["${var.subnets}"]
  enabled_metrics           = ["GroupMinSize", "GroupMaxSize", "GroupDesiredCapacity", "GroupInServiceInstances", "GroupPendingInstances", "GroupStandbyInstances", "GroupTerminatingInstances", "GroupTotalInstances"]

  tag {
    key                 = "Name"
    value               = "${var.app}-${var.env}"
    propagate_at_launch = true
  }

  tag {
    key                 = "app"
    value               = "${var.app}"
    propagate_at_launch = true
  }

  tag {
    key                 = "service"
    value               = "${var.service}"
    propagate_at_launch = true
  }

  tag {
    key                 = "env"
    value               = "${var.env}"
    propagate_at_launch = true
  }
}

resource "aws_autoscaling_policy" "highcpu" {
    name = "${var.app}-${var.env}-high-cpu-scaleup"
    scaling_adjustment = 2
    adjustment_type = "ChangeInCapacity"
    cooldown = 300
    autoscaling_group_name = "${aws_autoscaling_group.asg-green.name}"
}

resource "aws_cloudwatch_metric_alarm" "highcpu" {
    alarm_name = "${var.app}-${var.env}-high-cpu"
    comparison_operator = "GreaterThanOrEqualToThreshold"
    evaluation_periods = "2"
    metric_name = "CPUUtilization"
    namespace = "AWS/EC2"
    period = "120"
    statistic = "Average"
    threshold = "60"
    dimensions {
        AutoScalingGroupName = "${aws_autoscaling_group.asg-green.name}"
    }
    alarm_description = "Watch CPU usage for ${aws_autoscaling_group.asg-green.name} ASG"
    alarm_actions = ["${aws_autoscaling_policy.highcpu.arn}"]
}

resource "aws_autoscaling_policy" "lowcpu" {
    name = "${var.app}-${var.env}-low-cpu-scaledown"
    scaling_adjustment = -1
    adjustment_type = "ChangeInCapacity"
    cooldown = 300
    autoscaling_group_name = "${aws_autoscaling_group.asg-green.name}"
}

resource "aws_cloudwatch_metric_alarm" "lowcpu" {
    alarm_name = "${var.app}-${var.env}-low-cpu"
    comparison_operator = "LessThanOrEqualToThreshold"
    evaluation_periods = "2"
    metric_name = "CPUUtilization"
    namespace = "AWS/EC2"
    period = "120"
    statistic = "Average"
    threshold = "20"
    dimensions {
        AutoScalingGroupName = "${aws_autoscaling_group.asg-green.name}"
    }
    alarm_description = "Watch CPU usage for ${aws_autoscaling_group.asg-green.name} ASG"
    alarm_actions = ["${aws_autoscaling_policy.lowcpu.arn}"]
}